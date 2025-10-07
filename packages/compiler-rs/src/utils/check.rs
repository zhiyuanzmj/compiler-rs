use napi::{
  JsValue, ValueType,
  bindgen_prelude::{BigInt, JsObjectValue, Object},
};
use napi_derive::napi;

use crate::utils::{expression::is_globally_allowed, utils::unwrap_ts_node};

macro_rules! def_literal_checker {
  ($name:ident, $type:ty, $ts_return_type: literal) => {
    #[napi(ts_args_type = "node?: import('oxc-parser').Node | undefined | null", ts_return_type = $ts_return_type)]
    pub fn $name(node: Option<Object>) -> bool {
      let Some(node) = node else { return false };
      if let Ok(Some(type_value)) = node.get::<String>("type") {
        type_value.eq("Literal") && matches!(node.get::<$type>("value"), Ok(Some(_)))
      } else {
        false
      }
    }
  };
}

def_literal_checker!(
  is_string_literal,
  String,
  "node is import('oxc-parser').StringLiteral"
);
def_literal_checker!(
  is_big_int_literal,
  BigInt,
  "node is import('oxc-parser').BigIntLiteral"
);
def_literal_checker!(
  is_numeric_literal,
  f64,
  "node is import('oxc-parser').NumericLiteral"
);

#[napi(ts_args_type = "node?: import('oxc-parser').Node | undefined | null")]
pub fn is_template(node: Option<Object>) -> bool {
  let Some(node) = node else { return false };
  if !matches!(node.get::<String>("type"), Ok(Some(type_value)) if type_value.eq("JSXElement")) {
    return false;
  };
  if let Some(name) = node
    .get::<Object>("openingElement")
    .ok()
    .flatten()
    .and_then(|elem| elem.get::<Object>("name").ok().flatten())
  {
    matches!(name.get::<String>("type"), Ok(Some(type_value)) if type_value == "JSXIdentifier")
      && matches!(name.get::<String>("name"), Ok(Some(name)) if name == "template")
  } else {
    false
  }
}

#[napi]
pub fn is_constant_node(node: Option<Object>) -> bool {
  let Some(node) = node else {
    return false;
  };
  let node = unwrap_ts_node(node);
  let Some(node_type) = node.get_named_property::<String>("type").ok() else {
    return false;
  };
  if node_type == "UnaryExpression" {
    // void 0, !true
    is_constant_node(node.get::<Object>("argument").unwrap_or(None))
  } else if node_type == "LogicalExpression" || node_type == "BinaryExpression" {
    // 1 > 2, // 1 + 2
    is_constant_node(node.get::<Object>("left").unwrap_or(None))
      && is_constant_node(node.get::<Object>("right").unwrap_or(None))
  } else if node_type == "ConditionalExpression" {
    // 1 ? 2 : 3
    is_constant_node(node.get::<Object>("test").unwrap_or(None))
      && is_constant_node(node.get::<Object>("consequent").unwrap_or(None))
      && is_constant_node(node.get::<Object>("alternate").unwrap_or(None))
  } else if node_type == "SequenceExpression" || node_type == "TemplateLiteral" {
    // (1, 2) | `foo${1}`
    node
      .get::<Vec<Object>>("expressions")
      .unwrap()
      .unwrap()
      .into_iter()
      .all(|exp| is_constant_node(Some(exp)))
  } else if node_type == "ParenthesizedExpression" {
    is_constant_node(node.get::<Object>("expression").unwrap_or(None))
  } else if node_type == "Literal" {
    true
  } else if node_type == "Identifier" {
    let name = node
      .get_named_property::<String>("name")
      .unwrap_or(String::new());
    // .is_ok_and(|name| name.unwrap_or(String::new()).eq("undefined"));
    name == "undefined" || is_globally_allowed(&name)
  } else if node_type == "ObjectExpression" {
    let Some(props) = node.get_named_property::<Vec<Object>>("properties").ok() else {
      return false;
    };
    props.iter().all(|prop| {
      let name_type = prop
        .get_named_property::<String>("type")
        .unwrap_or(String::new());
      // { bar() {} } object methods are not considered static nodes
      if name_type == "Property"
        && prop
          .get_named_property::<bool>("method")
          .is_ok_and(|m| m == true)
      {
        return false;
      }
      // { ...{ foo: 1 } }
      if name_type == "SpreadElement" {
        return is_constant_node(prop.get_named_property::<Object>("argument").ok());
      }
      // { foo: 1 }
      (prop
        .get_named_property::<bool>("computed")
        .is_ok_and(|m| m != true)
        || is_constant_node(prop.get_named_property::<Object>("key").ok()))
        && is_constant_node(prop.get_named_property("value").ok())
    })
  } else if node_type == "ArrayExpression" {
    let Some(elements) = node.get_named_property::<Vec<Object>>("elements").ok() else {
      return false;
    };
    elements.iter().all(|element| {
      // [1, , 3]
      if let Ok(ValueType::Null) = element.to_unknown().get_type() {
        return true;
      }
      // [1, ...[2, 3]]
      if element
        .get_named_property::<String>("type")
        .is_ok_and(|t| t == "SpreadElement")
      {
        return is_constant_node(element.get_named_property("argument").ok());
      }
      // [1, 2]
      is_constant_node(Some(element.to_owned()))
    })
  } else {
    false
  }
}

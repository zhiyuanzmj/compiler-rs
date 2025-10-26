use napi::{
  Either,
  bindgen_prelude::{JsObjectValue, Object},
};
use napi_derive::napi;

use crate::utils::check::{is_big_int_literal, is_numeric_literal, is_string_literal};

#[napi]
pub const TS_NODE_TYPES: [&str; 5] = [
  "TSAsExpression",            // foo as number
  "TSTypeAssertion",           // (<number>foo)
  "TSNonNullExpression",       // foo!
  "TSInstantiationExpression", // foo<string>
  "TSSatisfiesExpression",     // foo satisfies T
];

#[napi(
  js_name = "unwrapTSNode",
  ts_args_type = "node: import('oxc-parser').Node",
  ts_return_type = "import('oxc-parser').Node"
)]
pub fn unwrap_ts_node(node: Object) -> Object {
  if let Ok(Some(type_value)) = node.get::<String>("type") {
    if TS_NODE_TYPES.contains(&type_value.as_str()) {
      node.get::<Object>("expression").unwrap().unwrap()
    } else {
      node
    }
  } else {
    node
  }
}

#[napi(
  ts_args_type = "node: import('oxc-parser').Node",
  ts_return_type = "import('oxc-parser').Node"
)]
pub fn get_expression(node: Object) -> Object {
  _get_expression(&node)
}
pub fn _get_expression<'a>(node: &Object<'a>) -> Object<'a> {
  let node = match node.get::<String>("type") {
    Ok(Some(t)) if t == "JSXExpressionContainer" => node
      .get::<Object>("expression")
      .ok()
      .flatten()
      .map_or(*node, |n| n),
    _ => *node,
  };
  let node = match node.get::<String>("type") {
    Ok(Some(t)) if t == "ParenthesizedExpression" => node
      .get::<Object>("expression")
      .ok()
      .flatten()
      .map_or(node, |n| n),
    _ => node,
  };
  node
}

#[napi]
pub fn get_text_like_value(
  #[napi(ts_arg_type = "import('oxc-parser').Node")] node: Object,
  exclude_number: Option<bool>,
) -> Option<String> {
  _get_text_like_value(&node, exclude_number)
}

pub fn _get_text_like_value(node: &Object, exclude_number: Option<bool>) -> Option<String> {
  let node = _get_expression(node);
  if is_string_literal(Some(node)) {
    return node.get::<String>("value").ok().flatten();
  } else if !exclude_number.unwrap_or(false)
    && (is_numeric_literal(Some(node)) || is_big_int_literal(Some(node)))
  {
    if is_numeric_literal(Some(node)) {
      return Some(node.get::<f64>("value").ok().flatten()?.to_string());
    } else {
      return node.get::<String>("bigint").ok().flatten();
    }
  } else if matches!(node.get::<String>("type"), Ok(Some(type_value)) if type_value == "TemplateLiteral")
  {
    let quasis = node.get_named_property::<Vec<Object>>("quasis").ok()?;
    let expressions = node.get_named_property::<Vec<Object>>("expressions").ok()?;
    let mut result = String::new();
    for i in 0..quasis.len() {
      result += &quasis[i]
        .get_named_property::<Object>("value")
        .ok()?
        .get_named_property::<String>("cooked")
        .ok()?;
      if let Some(expression) = expressions.get(i) {
        let Some(expression_value) = _get_text_like_value(expression, None) else {
          return None;
        };
        result += &expression_value;
      }
    }
    return Some(result);
  }
  None
}

pub fn find_prop<'a>(node: &'a Object, key: Either<String, Vec<String>>) -> Option<Object<'a>> {
  if node
    .get_named_property::<String>("type")
    .is_ok_and(|t| t.eq("JSXElement"))
  {
    let attributes = node
      .get::<Object>("openingElement")
      .ok()??
      .get::<Vec<Object>>("attributes")
      .ok()??;

    for attr in attributes {
      if attr.get::<String>("type").ok()??.eq("JSXAttribute") {
        let name = attr.get::<Object>("name").ok()??;
        let name_type = name.get::<String>("type").ok()??;
        let name = if name_type.eq("JSXIdentifier") {
          name.get::<String>("name").ok()??
        } else {
          name_type
            .eq("JSXNamespacedName")
            .then_some(
              name
                .get::<Object>("namespace")
                .ok()??
                .get::<String>("name")
                .ok()??,
            )
            .or(Some(String::from("")))?
        };
        let name = name.split('_').collect::<Vec<&str>>()[0];
        if !name.eq("")
          && match &key {
            Either::A(s) => s.eq(name),
            Either::B(s) => s.contains(&name.to_string()),
          }
        {
          return Some(attr);
        }
      }
    }
  }
  None
}

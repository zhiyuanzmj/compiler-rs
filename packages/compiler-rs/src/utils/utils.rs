use napi::Either;
use oxc_ast::ast::{Expression, JSXAttribute, JSXAttributeItem, JSXAttributeName, JSXElement};

pub fn get_text_like_value<'a>(
  node: &'a Expression,
  exclude_number: Option<bool>,
) -> Option<String> {
  let node = node.without_parentheses().get_inner_expression();
  if let Expression::StringLiteral(node) = node {
    return Some(node.value.to_string());
  } else if !exclude_number.unwrap_or(false) && node.is_number_literal() {
    if let Expression::NumericLiteral(node) = node {
      return Some(node.value.to_string());
    } else if let Expression::BigIntLiteral(node) = node {
      return Some(node.value.to_string());
    }
  } else if let Expression::TemplateLiteral(node) = node {
    let mut result = String::new();
    for i in 0..node.quasis.len() {
      result += &node.quasis[i].value.cooked.unwrap().to_string();
      if let Some(expression) = node.expressions.get(i) {
        let Some(expression_value) = get_text_like_value(expression, None) else {
          return None;
        };
        result += &expression_value;
      }
    }
    return Some(result);
  }
  None
}

macro_rules! define_find_prop {
  ($fn_name:ident, $node_type: ty, $ret_type: ty, $iter: tt) => {
    pub fn $fn_name<'a>(node: $node_type, key: Either<String, Vec<String>>) -> Option<$ret_type> {
      for attr in node.opening_element.attributes.$iter() {
        if let JSXAttributeItem::Attribute(attr) = attr {
          let name = match &attr.name {
            JSXAttributeName::Identifier(name) => name.name.to_string(),
            JSXAttributeName::NamespacedName(name) => name.namespace.name.to_string(),
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
      None
    }
  };
}
define_find_prop!(find_prop, &'a JSXElement<'a>, &'a JSXAttribute<'a>, iter);
define_find_prop!(
  find_prop_mut,
  &'a mut JSXElement<'a>,
  &'a mut JSXAttribute<'a>,
  iter_mut
);

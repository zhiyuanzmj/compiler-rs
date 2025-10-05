use napi::{JsValue, ValueType, bindgen_prelude::Object};
use napi_derive::napi;

use crate::{
  ir::index::{SimpleExpressionNode, SourceLocation},
  utils::{
    check::is_string_literal,
    text::{get_text, resolve_jsx_text},
    utils::{get_expression, unwrap_ts_node},
  },
};

#[napi(js_name = "locStub")]
pub const LOC_STUB: SourceLocation = (0, 0);

#[napi]
pub fn create_simple_expression(
  content: String,
  is_static: Option<bool>,
  ast: Option<Object<'static>>,
  loc: Option<SourceLocation>,
) -> SimpleExpressionNode {
  SimpleExpressionNode {
    content,
    is_static: is_static.unwrap_or(false),
    ast,
    loc: match loc {
      Some(loc) => Some(loc),
      None => ast.map_or(Some(LOC_STUB), |ast| {
        return ast.get::<SourceLocation>("range").ok().flatten();
      }),
    },
  }
}

#[napi]
pub const EMPTY_EXPRESSION: SimpleExpressionNode = SimpleExpressionNode {
  content: String::new(),
  is_static: true,
  ast: None,
  loc: None,
};

pub fn get_value<T: JsValue<'static>>(obj: Object) -> Option<T> {
  let result = obj.get::<T>("value").ok().flatten();
  if let Some(result) = result {
    if let Ok(ValueType::Null) = result.to_unknown().get_type() {
      return None;
    }
    Some(result)
  } else {
    None
  }
}

#[napi]
pub fn resolve_expression(
  #[napi(ts_arg_type = "import('oxc-parser').Node")] node: Object<'static>,
  context: Object,
) -> SimpleExpressionNode {
  let node = unwrap_ts_node(get_expression(node));
  let node_type = &node
    .get::<String>("type")
    .ok()
    .flatten()
    .unwrap_or(String::new());
  let is_static =
    is_string_literal(Some(node)) || node_type.eq("JSXText") || node_type.eq("JSXIdentifier");
  let content = if node_type.eq("JSXEmptyExpression") {
    String::new()
  } else if node_type.eq("JSXIdentifier") {
    node
      .get::<String>("name")
      .ok()
      .flatten()
      .unwrap_or(String::new())
  } else if is_string_literal(Some(node)) {
    node
      .get::<String>("value")
      .ok()
      .flatten()
      .unwrap_or(String::new())
  } else if node_type.eq("JSXText") {
    resolve_jsx_text(node)
  } else if node_type.eq("Identifier") {
    node
      .get::<String>("name")
      .ok()
      .flatten()
      .unwrap_or(String::new())
  } else {
    get_text(node, context)
  };
  SimpleExpressionNode {
    content,
    is_static,
    ast: Some(node),
    loc: None,
  }
}

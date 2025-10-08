use std::{collections::HashSet, sync::LazyLock};

use napi::{JsValue, ValueType, bindgen_prelude::Object};
use napi_derive::napi;

use crate::{
  ir::index::{SimpleExpressionNode, SourceLocation},
  utils::{
    check::is_string_literal,
    text::{_get_text, resolve_jsx_text},
    utils::{get_expression, get_text_like_value, unwrap_ts_node},
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
  _resolve_expression(node, &context)
}
pub fn _resolve_expression(node: Object<'static>, context: &Object) -> SimpleExpressionNode {
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
    _get_text(node, context)
  };
  SimpleExpressionNode {
    content,
    is_static,
    ast: Some(node),
    loc: None,
  }
}

static LITERAL_WHITELIST: [&str; 4] = ["true", "false", "null", "this"];
fn is_literal_whitelisted(key: &str) -> bool {
  LITERAL_WHITELIST.contains(&key)
}

static GLOBALLY_ALLOWED: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
  HashSet::from([
    "Infinity",
    "undefined",
    "NaN",
    "isFinite",
    "isNaN",
    "parseFloat",
    "parseInt",
    "decodeURI",
    "decodeURIComponent",
    "encodeURI",
    "encodeURIComponent",
    "Math",
    "Number",
    "Date",
    "Array",
    "Object",
    "Boolean",
    "String",
    "RegExp",
    "Map",
    "Set",
    "JSON",
    "Intl",
    "BigInt",
    "console",
    "Error",
    "Symbol",
  ])
});
pub fn is_globally_allowed(key: &str) -> bool {
  GLOBALLY_ALLOWED.contains(&key)
}

#[napi]
pub fn is_constant_expression(exp: SimpleExpressionNode) -> bool {
  _is_constant_expression(&exp)
}
pub fn _is_constant_expression(exp: &SimpleExpressionNode) -> bool {
  is_literal_whitelisted(&exp.content)
    || is_globally_allowed(&exp.content)
    || _get_literal_expression_value(exp).is_some()
}

#[napi]
pub fn get_literal_expression_value(exp: SimpleExpressionNode) -> Option<String> {
  _get_literal_expression_value(&exp)
}
pub fn _get_literal_expression_value(exp: &SimpleExpressionNode) -> Option<String> {
  if let Some(ast) = exp.ast {
    if let Some(res) = get_text_like_value(ast, None) {
      return Some(res);
    }
  }
  if exp.is_static {
    Some(exp.content.to_string())
  } else {
    None
  }
}

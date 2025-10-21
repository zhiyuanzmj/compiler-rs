use napi_derive::napi;
use std::{rc::Rc, sync::LazyLock};

use napi::{
  JsValue,
  bindgen_prelude::{JsObjectValue, Object},
};
use regex::{Captures, Regex};

use crate::transform::TransformContext;

static EMPTY_TEXT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"^[\t\v\f \u00A0\u1680\u2000-\u200A\u2028\u2029\u202F\u205F\u3000\uFEFF]*[\n\r]\s*$")
    .unwrap()
});

static START_EMPTY_TEXT_REGEX: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^\s*[\n\r]").unwrap());

static END_EMPTY_TEXT_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\n\r]\s*$").unwrap());

#[napi(
  js_name = "resolveJSXText",
  ts_args_type = "node: import('oxc-parser').JSXText"
)]
pub fn resolve_jsx_text(node: Object) -> String {
  let text = node
    .get::<String>("raw")
    .ok()
    .flatten()
    .map_or(String::new(), |s| s);
  if EMPTY_TEXT_REGEX.is_match(&text) {
    return String::new();
  }
  let mut value = node
    .get::<String>("value")
    .ok()
    .flatten()
    .unwrap_or(String::new());
  if START_EMPTY_TEXT_REGEX.is_match(&value) {
    value = value.trim_start().to_owned();
  }
  if END_EMPTY_TEXT_REGEX.is_match(&value) {
    value = value.trim_end().to_owned();
  }
  return value;
}

#[napi]
pub fn is_empty_text(node: Object) -> bool {
  let node_type = node
    .get::<String>("type")
    .ok()
    .flatten()
    .unwrap_or(String::new());
  let node_raw = node
    .get::<String>("raw")
    .ok()
    .flatten()
    .unwrap_or(String::new());
  (node_type.eq("JSXText") && EMPTY_TEXT_REGEX.is_match(&node_raw))
    || (node_type.eq("JSXExpressionContainer")
      && node
        .get::<Object>("expression")
        .ok()
        .flatten()
        .map_or(false, |e| {
          e.get::<String>("type")
            .ok()
            .flatten()
            .is_some_and(|t| t.eq("JSXEmptyExpression"))
        }))
}

pub fn get_text(node: Object, context: &Rc<TransformContext>) -> String {
  let start = node.get::<i32>("start").ok().flatten().unwrap() as usize;
  let end = node.get::<i32>("end").ok().flatten().unwrap() as usize;
  context.ir.borrow().source[start..end].to_string()
}

static CAMELIZE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-(\w)").unwrap());
#[napi]
pub fn camelize(str: String) -> String {
  CAMELIZE_RE
    .replace_all(&str, |caps: &Captures| caps[1].to_uppercase())
    .to_string()
}

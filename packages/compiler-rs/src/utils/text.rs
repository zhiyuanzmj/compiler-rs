use oxc_ast::ast::{JSXChild, JSXElementName, JSXExpression, JSXText};
use oxc_span::Span;
use std::{rc::Rc, sync::LazyLock};

use regex::{Captures, Regex};

use crate::transform::TransformContext;

static EMPTY_TEXT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"^[\t\v\f \u00A0\u1680\u2000-\u200A\u2028\u2029\u202F\u205F\u3000\uFEFF]*[\n\r]\s*$")
    .unwrap()
});

static START_EMPTY_TEXT_REGEX: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^\s*[\n\r]").unwrap());

static END_EMPTY_TEXT_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\n\r]\s*$").unwrap());

pub fn resolve_jsx_text(node: &JSXText) -> String {
  if EMPTY_TEXT_REGEX.is_match(&node.raw.unwrap()) {
    return String::new();
  }
  let mut value = node.value.to_string();
  if START_EMPTY_TEXT_REGEX.is_match(&value) {
    value = value.trim_start().to_owned();
  }
  if END_EMPTY_TEXT_REGEX.is_match(&value) {
    value = value.trim_end().to_owned();
  }
  return value;
}

pub fn is_empty_text(node: &JSXChild) -> bool {
  match node {
    JSXChild::Text(node) => EMPTY_TEXT_REGEX.is_match(&node.raw.unwrap()),
    JSXChild::ExpressionContainer(node) => {
      matches!(node.expression, JSXExpression::EmptyExpression(_))
    }
    _ => false,
  }
}

pub fn get_tag_name(name: &JSXElementName, context: &TransformContext) -> String {
  match name {
    JSXElementName::Identifier(name) => name.name.to_string(),
    JSXElementName::IdentifierReference(name) => name.name.to_string(),
    JSXElementName::NamespacedName(name) => {
      context.ir.borrow().source[name.span.start as usize..name.span.end as usize].to_string()
    }
    JSXElementName::MemberExpression(name) => {
      context.ir.borrow().source[name.span.start as usize..name.span.end as usize].to_string()
    }
    JSXElementName::ThisExpression(name) => {
      context.ir.borrow().source[name.span.start as usize..name.span.end as usize].to_string()
    }
  }
}

pub fn get_text(span: Span, context: &Rc<TransformContext>) -> String {
  let start = span.start as usize;
  let end = span.end as usize;
  context.ir.borrow().source[start..end].to_string()
}

static CAMELIZE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-(\w)").unwrap());
pub fn camelize(str: String) -> String {
  CAMELIZE_RE
    .replace_all(&str, |caps: &Captures| caps[1].to_uppercase())
    .to_string()
}

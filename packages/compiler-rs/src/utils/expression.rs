use std::{collections::HashSet, sync::LazyLock};

use napi::bindgen_prelude::Either3;

use oxc_allocator::CloneIn;
use oxc_ast::ast::{Expression, JSXAttributeValue, JSXChild, JSXExpression};
use oxc_span::{GetSpan, Span};

use crate::{
  transform::TransformContext,
  utils::{text::resolve_jsx_text, utils::get_text_like_value},
};

pub type SourceLocation = Span;

pub const LOC_STUB: LazyLock<Span> = LazyLock::new(|| Span::new(0, 0));

#[derive(Debug)]
pub struct SimpleExpressionNode<'a> {
  pub content: String,
  pub is_static: bool,
  pub loc: Option<SourceLocation>,
  pub ast: Option<Expression<'a>>,
}

impl<'a> Clone for SimpleExpressionNode<'a> {
  fn clone(&self) -> Self {
    Self {
      content: self.content.clone(),
      is_static: self.is_static,
      loc: self.loc.clone(),
      ast: None,
    }
  }
}

impl<'a> Default for SimpleExpressionNode<'a> {
  fn default() -> Self {
    Self {
      content: String::new(),
      is_static: true,
      loc: None,
      ast: None,
    }
  }
}

impl<'a> SimpleExpressionNode<'a> {
  pub fn new(
    node: Either3<&Expression, &JSXChild, &JSXAttributeValue>,
    context: &TransformContext<'a>,
  ) -> SimpleExpressionNode<'a> {
    let mut is_static = false;
    let mut ast = None;
    let content = match node {
      Either3::A(node) => {
        ast = Some(node.clone_in(context.allocator));
        let span = node.span();
        context.ir.borrow().source[span.start as usize..span.end as usize].to_string()
      }
      Either3::B(node) => match node {
        JSXChild::ExpressionContainer(node) => {
          let expression = node.expression.to_expression();
          ast = Some(expression.clone_in(context.allocator));
          let span = expression.span();
          context.ir.borrow().source[span.start as usize..span.end as usize].to_string()
        }
        JSXChild::Text(node) => {
          is_static = true;
          resolve_jsx_text(node)
        }
        JSXChild::Element(node) => {
          context.ir.borrow().source[node.span.start as usize..node.span.end as usize].to_string()
        }
        JSXChild::Fragment(node) => {
          context.ir.borrow().source[node.span.start as usize..node.span.end as usize].to_string()
        }
        JSXChild::Spread(node) => {
          context.ir.borrow().source[node.span.start as usize..node.span.end as usize].to_string()
        }
      },
      Either3::C(node) => match node {
        JSXAttributeValue::ExpressionContainer(node) => {
          let expression = node.expression.to_expression();
          ast = Some(expression.clone_in(context.allocator));
          is_static = matches!(expression, Expression::StringLiteral(_));
          let span = expression.span();
          context.ir.borrow().source[span.start as usize..span.end as usize].to_string()
        }
        JSXAttributeValue::StringLiteral(node) => {
          is_static = true;
          ast = Some(Expression::StringLiteral(node.clone_in(context.allocator)));
          node.value.to_string()
        }
        JSXAttributeValue::Element(node) => {
          context.ir.borrow().source[node.span.start as usize..node.span.end as usize].to_string()
        }
        JSXAttributeValue::Fragment(node) => {
          context.ir.borrow().source[node.span.start as usize..node.span.end as usize].to_string()
        }
      },
    };
    SimpleExpressionNode {
      content,
      is_static,
      ast,
      loc: None,
    }
  }

  pub fn is_constant_expression(&self) -> bool {
    is_literal_whitelisted(&self.content)
      || is_globally_allowed(&self.content)
      || self.get_literal_expression_value().is_some()
  }

  pub fn get_literal_expression_value(&self) -> Option<String> {
    if let Some(ast) = &self.ast {
      if let Some(res) = get_text_like_value(ast, None) {
        return Some(res);
      }
    }
    if self.is_static {
      Some(self.content.to_string())
    } else {
      None
    }
  }
}

pub fn to_jsx_expression<'a>(expression: Expression<'a>) -> JSXExpression<'a> {
  match expression {
    Expression::BooleanLiteral(e) => JSXExpression::BooleanLiteral(e),
    Expression::NullLiteral(e) => JSXExpression::NullLiteral(e),
    Expression::NumericLiteral(e) => JSXExpression::NumericLiteral(e),
    Expression::BigIntLiteral(e) => JSXExpression::BigIntLiteral(e),
    Expression::RegExpLiteral(e) => JSXExpression::RegExpLiteral(e),
    Expression::StringLiteral(e) => JSXExpression::StringLiteral(e),
    Expression::TemplateLiteral(e) => JSXExpression::TemplateLiteral(e),
    Expression::Identifier(e) => JSXExpression::Identifier(e),
    Expression::MetaProperty(e) => JSXExpression::MetaProperty(e),
    Expression::Super(e) => JSXExpression::Super(e),
    Expression::ArrayExpression(e) => JSXExpression::ArrayExpression(e),
    Expression::ArrowFunctionExpression(e) => JSXExpression::ArrowFunctionExpression(e),
    Expression::AssignmentExpression(e) => JSXExpression::AssignmentExpression(e),
    Expression::AwaitExpression(e) => JSXExpression::AwaitExpression(e),
    Expression::BinaryExpression(e) => JSXExpression::BinaryExpression(e),
    Expression::CallExpression(e) => JSXExpression::CallExpression(e),
    Expression::ChainExpression(e) => JSXExpression::ChainExpression(e),
    Expression::ClassExpression(e) => JSXExpression::ClassExpression(e),
    Expression::ComputedMemberExpression(e) => JSXExpression::ComputedMemberExpression(e),
    Expression::ConditionalExpression(e) => JSXExpression::ConditionalExpression(e),
    Expression::FunctionExpression(e) => JSXExpression::FunctionExpression(e),
    Expression::ImportExpression(e) => JSXExpression::ImportExpression(e),
    Expression::LogicalExpression(e) => JSXExpression::LogicalExpression(e),
    Expression::NewExpression(e) => JSXExpression::NewExpression(e),
    Expression::ObjectExpression(e) => JSXExpression::ObjectExpression(e),
    Expression::ParenthesizedExpression(e) => JSXExpression::ParenthesizedExpression(e),
    Expression::PrivateFieldExpression(e) => JSXExpression::PrivateFieldExpression(e),
    Expression::StaticMemberExpression(e) => JSXExpression::StaticMemberExpression(e),
    Expression::SequenceExpression(e) => JSXExpression::SequenceExpression(e),
    Expression::TaggedTemplateExpression(e) => JSXExpression::TaggedTemplateExpression(e),
    Expression::ThisExpression(e) => JSXExpression::ThisExpression(e),
    Expression::UnaryExpression(e) => JSXExpression::UnaryExpression(e),
    Expression::UpdateExpression(e) => JSXExpression::UpdateExpression(e),
    Expression::YieldExpression(e) => JSXExpression::YieldExpression(e),
    Expression::PrivateInExpression(e) => JSXExpression::PrivateInExpression(e),
    Expression::JSXElement(e) => JSXExpression::JSXElement(e),
    Expression::JSXFragment(e) => JSXExpression::JSXFragment(e),
    Expression::TSAsExpression(e) => JSXExpression::TSAsExpression(e),
    Expression::TSSatisfiesExpression(e) => JSXExpression::TSSatisfiesExpression(e),
    Expression::TSTypeAssertion(e) => JSXExpression::TSTypeAssertion(e),
    Expression::TSNonNullExpression(e) => JSXExpression::TSNonNullExpression(e),
    Expression::TSInstantiationExpression(e) => JSXExpression::TSInstantiationExpression(e),
    Expression::V8IntrinsicExpression(e) => JSXExpression::V8IntrinsicExpression(e),
  }
}

static LITERAL_WHITELIST: [&str; 4] = ["true", "false", "null", "this"];
pub fn is_literal_whitelisted(key: &str) -> bool {
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

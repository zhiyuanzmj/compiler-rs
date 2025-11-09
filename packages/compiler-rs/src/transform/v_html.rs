use napi::bindgen_prelude::{Either3, Either16};
use oxc_ast::ast::{JSXAttribute, JSXElement};

use crate::{
  ir::index::{BlockIRNode, SetHtmlIRNode, SimpleExpressionNode},
  transform::{DirectiveTransformResult, TransformContext},
  utils::error::{ErrorCodes, on_error},
};

pub fn transform_v_html<'a>(
  dir: &JSXAttribute,
  node: &JSXElement,
  context: &'a TransformContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
) -> Option<DirectiveTransformResult<'a>> {
  let exp = if let Some(value) = &dir.value {
    SimpleExpressionNode::new(Either3::C(value), context)
  } else {
    on_error(ErrorCodes::VHtmlNoExpression, context);
    SimpleExpressionNode::default()
  };

  if node.children.len() != 0 {
    on_error(ErrorCodes::VHtmlWithChildren, context);
    return None;
  }

  let element = context.reference(&mut context_block.dynamic);
  context.register_effect(
    context_block,
    context.is_operation(vec![&exp]),
    Either16::I(SetHtmlIRNode {
      set_html: true,
      element,
      value: exp,
    }),
    None,
    None,
  );
  None
}

use napi::{
  Either,
  bindgen_prelude::{Either3, Either4},
};
use oxc_ast::ast::{JSXAttributeValue, JSXChild};

use crate::{
  ir::{
    component::{IRSlotType, IRSlotsExpression},
    index::{BlockIRNode, SimpleExpressionNode},
  },
  transform::TransformContext,
  utils::{
    check::is_jsx_component,
    error::{ErrorCodes, on_error},
    utils::find_prop,
  },
};

pub fn transform_v_slots<'a>(
  node: &JSXChild,
  context: &'a TransformContext<'a>,
  _: &'a mut BlockIRNode<'a>,
) -> Option<Box<dyn FnOnce() + 'a>> {
  if let JSXChild::Element(node) = node
    && is_jsx_component(node)
    && let Some(dir) = find_prop(node, Either::A("v-slots".to_string()))
  {
    if let Some(JSXAttributeValue::ExpressionContainer(value)) = &dir.value {
      let slots = SimpleExpressionNode::new(Either3::A(value.expression.to_expression()), context);
      Some(Box::new(move || {
        *context.slots.borrow_mut() = vec![Either4::D(IRSlotsExpression {
          slot_type: IRSlotType::EXPRESSION,
          slots,
        })];
      }))
    } else {
      on_error(ErrorCodes::VSlotMisplaced, context);
      None
    }
  } else {
    None
  }
}

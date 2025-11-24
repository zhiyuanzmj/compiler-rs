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
  transform::{ContextNode, TransformContext},
  utils::{check::is_jsx_component, error::ErrorCodes, utils::find_prop_mut},
};

pub fn transform_v_slots<'a>(
  context_node: &'a mut ContextNode<'a>,
  context: &'a TransformContext<'a>,
  _: &'a mut BlockIRNode<'a>,
  _: &'a mut ContextNode<'a>,
) -> Option<Box<dyn FnOnce() + 'a>> {
  if let Either::B(JSXChild::Element(node)) = context_node
    && is_jsx_component(node)
    && let Some(dir) = find_prop_mut(node, Either::A("v-slots".to_string()))
  {
    if let Some(JSXAttributeValue::ExpressionContainer(value)) = &mut dir.value {
      let slots =
        SimpleExpressionNode::new(Either3::A(value.expression.to_expression_mut()), context);
      Some(Box::new(move || {
        *context.slots.borrow_mut() = vec![Either4::D(IRSlotsExpression {
          slot_type: IRSlotType::EXPRESSION,
          slots,
        })];
      }))
    } else {
      context.options.on_error.as_ref()(ErrorCodes::VSlotMisplaced);
      None
    }
  } else {
    None
  }
}

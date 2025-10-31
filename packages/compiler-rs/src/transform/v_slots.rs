use std::rc::Rc;

use napi::{
  Result,
  bindgen_prelude::{Either4, JsObjectValue, Object},
};

use crate::{
  ir::{
    component::{IRSlotType, IRSlotsExpression},
    index::BlockIRNode,
  },
  transform::{DirectiveTransformResult, TransformContext},
  utils::{
    check::is_jsx_component,
    error::{ErrorCodes, on_error},
    expression::resolve_expression,
  },
};

pub fn transform_v_slots(
  dir: Object,
  node: Object,
  context: &Rc<TransformContext>,
  _: &mut BlockIRNode,
) -> Result<Option<DirectiveTransformResult>> {
  if is_jsx_component(node)
    && dir
      .get_named_property::<Object>("value")?
      .get_named_property::<String>("type")?
      .eq("JSXExpressionContainer")
  {
    *context.slots.borrow_mut() = vec![Either4::D(IRSlotsExpression {
      slot_type: IRSlotType::EXPRESSION,
      slots: resolve_expression(
        dir
          .get_named_property::<Object>("value")?
          .get_named_property("expression")?,
        context,
      ),
    })];
  } else {
    on_error(ErrorCodes::X_V_SLOT_MISPLACED, context)
  }
  Ok(None)
}

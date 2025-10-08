use napi::{
  Env, Result,
  bindgen_prelude::{JsObjectValue, Object},
};
use napi_derive::napi;

use crate::{
  ir::component::{IRSlotType, IRSlotsExpression},
  utils::{
    check::is_jsx_component,
    error::{ErrorCodes, on_error},
    expression::resolve_expression,
  },
};

#[napi]
pub fn transform_v_slots(env: Env, dir: Object, node: Object, mut context: Object) -> Result<()> {
  if is_jsx_component(node)
    && dir
      .get_named_property::<Object>("value")?
      .get_named_property::<String>("type")?
      .eq("JSXExpressionContainer")
  {
    context.set_named_property(
      "slots",
      vec![IRSlotsExpression {
        slot_type: IRSlotType::EXPRESSION,
        slots: resolve_expression(
          dir
            .get_named_property::<Object>("value")?
            .get_named_property("expression")?,
          context,
        ),
      }],
    )?
  } else {
    on_error(env, ErrorCodes::X_V_SLOT_MISPLACED, context)
  }
  Ok(())
}

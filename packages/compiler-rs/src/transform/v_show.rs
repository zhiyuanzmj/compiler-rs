use napi::{
  Env, Result,
  bindgen_prelude::{Function, JsObjectValue, Object},
};
use napi_derive::napi;

use crate::{
  ir::index::{DirectiveIRNode, IRNodeTypes},
  utils::{
    directive::resolve_directive,
    error::{ErrorCodes, on_error},
    expression::EMPTY_EXPRESSION,
  },
};

#[napi]
pub fn transform_v_show(env: Env, _dir: Object, _: Object, context: Object) -> Result<()> {
  let mut dir = resolve_directive(_dir, context)?;
  if dir.exp.is_none() {
    on_error(env, ErrorCodes::X_V_SHOW_NO_EXPRESSION, context);
    dir.exp = Some(EMPTY_EXPRESSION)
  }

  context
    .get_named_property::<Function<DirectiveIRNode, ()>>("registerOperation")?
    .apply(
      context,
      DirectiveIRNode {
        _type: IRNodeTypes::DIRECTIVE,
        element: context
          .get_named_property::<Function<(), i32>>("reference")?
          .apply(context, ())?,
        dir,
        name: String::from("show"),
        builtin: Some(true),
        asset: None,
        model_type: None,
      },
    )?;
  Ok(())
}

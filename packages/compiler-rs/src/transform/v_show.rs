use napi::{
  Env, Result,
  bindgen_prelude::{Either18, Object},
};

use crate::{
  ir::index::{DirectiveIRNode, IRNodeTypes},
  transform::{DirectiveTransformResult, reference, register_operation},
  utils::{
    directive::resolve_directive,
    error::{ErrorCodes, on_error},
    expression::EMPTY_EXPRESSION,
  },
};

pub fn transform_v_show(
  env: Env,
  _dir: Object,
  _: Object,
  context: Object,
) -> Result<Option<DirectiveTransformResult>> {
  let mut dir = resolve_directive(_dir, context)?;
  if dir.exp.is_none() {
    on_error(env, ErrorCodes::X_V_SHOW_NO_EXPRESSION, context);
    dir.exp = Some(EMPTY_EXPRESSION)
  }

  register_operation(
    &context,
    Either18::N(DirectiveIRNode {
      _type: IRNodeTypes::DIRECTIVE,
      element: reference(context)?,
      dir,
      name: String::from("show"),
      builtin: Some(true),
      asset: None,
      model_type: None,
    }),
    None,
  )?;
  Ok(None)
}

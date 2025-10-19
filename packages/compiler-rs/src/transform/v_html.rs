use napi::{
  Env, Result,
  bindgen_prelude::{Either18, JsObjectValue, Object},
};
use napi_derive::napi;

use crate::{
  ir::index::{IRNodeTypes, SetHtmlIRNode},
  transform::{DirectiveTransformResult, is_operation, reference, register_effect},
  utils::{
    error::{ErrorCodes, on_error},
    expression::{EMPTY_EXPRESSION, get_value, resolve_expression},
  },
};

pub fn transform_v_html(
  env: Env,
  dir: Object,
  node: Object,
  context: Object,
) -> Result<Option<DirectiveTransformResult>> {
  let exp = if let Some(value) = get_value::<Object>(dir) {
    resolve_expression(value, context)
  } else {
    on_error(env, ErrorCodes::X_V_HTML_NO_EXPRESSION, context);
    EMPTY_EXPRESSION
  };

  if let Some(children) = node.get_named_property::<Vec<Object>>("children").ok() {
    if children.len() != 0 {
      on_error(env, ErrorCodes::X_V_HTML_WITH_CHILDREN, context);
    }
    unsafe {
      context
        .get_named_property::<Vec<String>>("childrenTemplate")?
        .set_len(0);
    }
  }

  register_effect(
    &context,
    is_operation(vec![&exp], &context),
    Either18::I(SetHtmlIRNode {
      _type: IRNodeTypes::SET_HTML,
      element: reference(context)?,
      value: exp,
    }),
    None,
    None,
  )?;

  Ok(None)
}

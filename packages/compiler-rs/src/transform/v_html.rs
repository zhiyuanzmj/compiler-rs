use napi::{
  Env, Result,
  bindgen_prelude::{FnArgs, Function, JsObjectValue, Object},
};
use napi_derive::napi;

use crate::{
  ir::index::{IRNodeTypes, SetHtmlIRNode},
  transform::is_operation,
  utils::{
    error::{ErrorCodes, on_error},
    expression::{EMPTY_EXPRESSION, get_value, resolve_expression},
  },
};

#[napi]
pub fn transform_v_html(env: Env, dir: Object, node: Object, context: Object) -> Result<()> {
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
    if let Ok(mut children_template) = context.get_named_property::<Vec<String>>("childrenTemplate")
    {
      unsafe {
        children_template.set_len(0);
      }
    }
  }

  context
    .get_named_property::<Function<FnArgs<(bool, SetHtmlIRNode)>, ()>>("registerEffect")?
    .apply(
      context,
      FnArgs::from((
        is_operation(vec![&exp], &context),
        SetHtmlIRNode {
          _type: IRNodeTypes::SET_HTML,
          element: context
            .get_named_property::<Function<(), i32>>("reference")?
            .apply(context, ())?,
          value: exp,
        },
      )),
    )?;

  Ok(())
}

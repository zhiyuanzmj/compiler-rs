use napi::{
  Env, Result,
  bindgen_prelude::{FnArgs, Function, JsObjectValue, Object},
};
use napi_derive::napi;

use crate::{
  ir::index::{GetTextChildIRNode, IRNodeTypes, SetTextIRNode},
  transform::is_operation,
  utils::{
    check::is_void_tag,
    error::{ErrorCodes, on_error},
    expression::{_get_literal_expression_value, EMPTY_EXPRESSION, resolve_expression},
    text::get_text,
  },
};

#[napi]
pub fn transform_v_text(
  env: Env,
  #[napi(ts_arg_type = "import('oxc-parser').JSXAttribute")] dir: Object,
  #[napi(ts_arg_type = "import('oxc-parser').JSXElement")] node: Object,
  mut context: Object,
) -> Result<()> {
  let exp = if let Ok(value) = dir.get_named_property::<Object>("value") {
    resolve_expression(value, context)
  } else {
    on_error(env, ErrorCodes::X_V_TEXT_NO_EXPRESSION, context);
    EMPTY_EXPRESSION
  };

  if !node
    .get_named_property::<Vec<Object>>("children")?
    .is_empty()
  {
    on_error(env, ErrorCodes::X_V_TEXT_WITH_CHILDREN, context);
    unsafe {
      context
        .get_named_property::<Vec<String>>("childrenTemplate")?
        .set_len(0);
    }
  };

  // v-text on void tags do nothing
  if is_void_tag(&get_text(
    node
      .get_named_property::<Object>("openingElement")?
      .get_named_property("name")?,
    context,
  )) {
    return Ok(());
  }

  let literal = _get_literal_expression_value(&exp);
  if let Some(literal) = literal {
    context.set("childrenTemplate", vec![literal])?
  } else {
    context.set("childrenTemplate", [" "])?;
    let reference = context.get_named_property::<Function<(), i32>>("reference")?;
    context
      .get_named_property::<Function<GetTextChildIRNode, ()>>("registerOperation")?
      .apply(
        context,
        GetTextChildIRNode {
          _type: IRNodeTypes::GET_TEXT_CHILD,
          parent: reference.apply(context, ())?,
        },
      )?;
    context
      .get_named_property::<Function<FnArgs<(bool, SetTextIRNode)>, ()>>("registerEffect")?
      .apply(
        context,
        FnArgs::from((
          is_operation(vec![&exp], &context),
          SetTextIRNode {
            _type: IRNodeTypes::SET_TEXT,
            element: reference.apply(context, ())?,
            values: vec![exp],
            generated: Some(true),
          },
        )),
      )?
  }

  Ok(())
}

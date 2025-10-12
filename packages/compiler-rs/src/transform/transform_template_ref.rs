use napi::{
  Either, Env, Result,
  bindgen_prelude::{FnArgs, Function, JsObjectValue, Object},
};
use napi_derive::napi;

use crate::{
  ir::index::{DeclareOldRefIRNode, IRNodeTypes, SetTemplateRefIRNode},
  transform::is_operation,
  utils::{
    check::is_fragment_node,
    expression::{_is_constant_expression, _resolve_expression, EMPTY_EXPRESSION},
    utils::find_prop,
  },
};

#[napi]
pub fn transform_template_ref<'a>(
  env: &'a Env,
  node: Object<'static>,
  context: Object,
) -> Result<Option<Function<'a, (), ()>>> {
  if is_fragment_node(node) {
    return Ok(None);
  }

  let Some(dir) = find_prop(node, Either::A(String::from("ref"))) else {
    return Ok(None);
  };
  let Ok(value) = dir.get_named_property::<Object>("value") else {
    return Ok(None);
  };
  context
    .get_named_property::<Object>("ir")?
    .set("hasTemplateRef", true)?;

  Ok(Some(env.create_function_from_closure("cb", move |e| {
    let context = &e.first_arg::<Object>()?;
    let node = context.get_named_property::<Object>("node")?;
    let value = find_prop(node, Either::A(String::from("ref")))
      .unwrap()
      .get_named_property::<Object>("value")?;
    let value = _resolve_expression(value, &context);

    let id = context
      .get_named_property::<Function<(), i32>>("reference")?
      .apply(context, ())?;
    let effect = !_is_constant_expression(&value);
    if effect {
      context
        .get_named_property::<Function<DeclareOldRefIRNode, ()>>("registerOperation")?
        .apply(
          context,
          DeclareOldRefIRNode {
            _type: IRNodeTypes::DECLARE_OLD_REF,
            id,
          },
        )?;
    }

    context
      .get_named_property::<Function<FnArgs<(bool, SetTemplateRefIRNode)>, ()>>("registerEffect")?
      .apply(
        context,
        FnArgs::from((
          is_operation(vec![&value], context),
          SetTemplateRefIRNode {
            _type: IRNodeTypes::SET_TEMPLATE_REF,
            element: id,
            value,
            ref_for: context.get::<i32>("inVFor")?.is_some_and(|i| i != 0),
            effect,
          },
        )),
      )?;
    Ok(())
  })?))
}

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
  node: Object<'a>,
  context: Object<'a>,
) -> Result<Option<Function<'a, Object<'a>, ()>>> {
  if is_fragment_node(node) {
    return Ok(None);
  }

  if let Some(dir) = find_prop(node, Either::A(String::from("ref"))) {
    let Ok(value) = dir.get_named_property::<Object>("value") else {
      return Ok(None);
    };
    context
      .get_named_property::<Object>("ir")?
      .set("hasTemplateRef", true)?;

    let value = _resolve_expression(value, &context);

    return Ok(Some(env.create_function_from_closure("cb", move |e| {
      let context = &e.first_arg::<Object>()?;
      let id = context
        .get_named_property::<Function<(), i32>>("reference")?
        .apply(context, ())?;
      let effect = !_is_constant_expression(&value);
      if effect {
        context
          .get_named_property::<Object>("block")?
          .get_named_property::<Vec<DeclareOldRefIRNode>>("operation")?
          .push(DeclareOldRefIRNode {
            _type: IRNodeTypes::DECLARE_OLD_REF,
            id,
          });
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
              value: EMPTY_EXPRESSION,
              ref_for: context.get::<i32>("inVFor")?.is_some_and(|i| i != 0),
              effect,
            },
          )),
        )?;
      Ok(())
    })?));
  };

  Ok(None)
}

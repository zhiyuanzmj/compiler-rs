use napi::{
  Either, Env, Result,
  bindgen_prelude::{Either18, JsObjectValue, Object},
};

use crate::{
  ir::index::{DeclareOldRefIRNode, IRNodeTypes, SetTemplateRefIRNode},
  transform::{is_operation, reference, register_effect, register_operation},
  utils::{
    check::is_fragment_node,
    expression::{_is_constant_expression, _resolve_expression},
    utils::find_prop,
  },
};

pub fn transform_template_ref(
  _: Env,
  node: Object<'static>,
  context: Object<'static>,
) -> Result<Option<Box<dyn FnOnce() -> Result<()>>>> {
  if is_fragment_node(node) {
    return Ok(None);
  }

  let Some(dir) = find_prop(node, Either::A(String::from("ref"))) else {
    return Ok(None);
  };
  let Ok(_) = dir.get_named_property::<Object>("value") else {
    return Ok(None);
  };
  context
    .get_named_property::<Object>("ir")?
    .set("hasTemplateRef", true)?;

  Ok(Some(Box::new(move || {
    let node = context.get_named_property::<Object>("node")?;
    let value = find_prop(node, Either::A(String::from("ref")))
      .unwrap()
      .get_named_property::<Object>("value")?;
    let value = _resolve_expression(value, &context);

    let id = reference(context)?;
    let effect = !_is_constant_expression(&value);
    if effect {
      register_operation(
        &context,
        Either18::P(DeclareOldRefIRNode {
          _type: IRNodeTypes::DECLARE_OLD_REF,
          id,
        }),
        None,
      )?;
    }

    register_effect(
      &context,
      is_operation(vec![&value], &context),
      Either18::J(SetTemplateRefIRNode {
        _type: IRNodeTypes::SET_TEMPLATE_REF,
        element: id,
        value,
        ref_for: context.get::<i32>("inVFor")?.is_some_and(|i| i != 0),
        effect,
      }),
      None,
      None,
    )?;
    Ok(())
  })))
}

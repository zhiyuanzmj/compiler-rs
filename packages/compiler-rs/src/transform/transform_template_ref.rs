use std::rc::Rc;

use napi::{
  Either, Result,
  bindgen_prelude::{Either18, JsObjectValue, Object},
};

use crate::{
  ir::index::{BlockIRNode, DeclareOldRefIRNode, IRDynamicInfo, IRNodeTypes, SetTemplateRefIRNode},
  transform::TransformContext,
  utils::{
    check::is_fragment_node,
    expression::{_is_constant_expression, resolve_expression},
    utils::find_prop,
  },
};

pub fn transform_template_ref<'a>(
  node: Object<'static>,
  context: &'a Rc<TransformContext>,
  context_block: &'a mut BlockIRNode,
  _: &'a mut IRDynamicInfo,
) -> Result<Option<Box<dyn FnOnce() -> Result<()> + 'a>>> {
  if is_fragment_node(&node) {
    return Ok(None);
  }

  let Some(dir) = find_prop(&node, Either::A(String::from("ref"))) else {
    return Ok(None);
  };
  let Ok(_) = dir.get_named_property::<Object>("value") else {
    return Ok(None);
  };
  context.ir.borrow_mut().has_template_ref = true;

  Ok(Some(Box::new(move || {
    let value = find_prop(&node, Either::A(String::from("ref")))
      .unwrap()
      .get_named_property::<Object>("value")?;
    let value = resolve_expression(value, &context);

    let id = context.reference(&mut context_block.dynamic)?;
    let effect = !_is_constant_expression(&value);
    if effect {
      context.register_operation(
        context_block,
        Either18::P(DeclareOldRefIRNode {
          _type: IRNodeTypes::DECLARE_OLD_REF,
          id,
        }),
        None,
      )?;
    }

    context.register_effect(
      context_block,
      context.is_operation(vec![&value]),
      Either18::J(SetTemplateRefIRNode {
        _type: IRNodeTypes::SET_TEMPLATE_REF,
        element: id,
        value,
        ref_for: *context.in_v_for.borrow() != 0,
        effect,
      }),
      None,
      None,
    )?;
    Ok(())
  })))
}

use std::rc::Rc;

use napi::{Either, Result, bindgen_prelude::Object};

use crate::{
  ir::index::{BlockIRNode, IRDynamicInfo},
  transform::TransformContext,
  utils::utils::find_prop,
};

pub fn transform_v_once<'a>(
  node: Object<'static>,
  context: &'a Rc<TransformContext>,
  _: &'a mut BlockIRNode,
  _: &'a mut IRDynamicInfo,
) -> Result<Option<Box<dyn FnOnce() -> Result<()> + 'a>>> {
  if node
    .get::<String>("type")
    .is_ok_and(|x| x.is_some_and(|i| i.eq("JSXElement")))
    && find_prop(&node, Either::A(String::from("v-once"))).is_some()
  {
    *context.in_v_once.borrow_mut() = true;
  }
  Ok(None)
}

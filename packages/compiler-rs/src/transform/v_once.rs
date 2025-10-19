use napi::{Either, Env, Result, bindgen_prelude::Object};

use crate::utils::utils::find_prop;

pub fn transform_v_once(
  _: Env,
  node: Object<'static>,
  mut context: Object<'static>,
) -> Result<Option<Box<dyn FnOnce() -> Result<()>>>> {
  if node
    .get::<String>("type")
    .is_ok_and(|x| x.is_some_and(|i| i.eq("JSXElement")))
    && find_prop(node, Either::A(String::from("v-once"))).is_some()
  {
    context.set("inVOnce", true)?;
  }
  Ok(None)
}

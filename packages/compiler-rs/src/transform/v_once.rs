use napi::{Either, Result, bindgen_prelude::Object};
use napi_derive::napi;

use crate::utils::utils::find_prop;

#[napi]
pub fn transform_v_once(node: Object, mut context: Object) -> Result<()> {
  if node
    .get::<String>("type")
    .is_ok_and(|x| x.is_some_and(|i| i.eq("JSXElement")))
    && find_prop(node, Either::A(String::from("v-once"))).is_some()
  {
    return context.set("inVOnce", true);
  }
  Ok(())
}

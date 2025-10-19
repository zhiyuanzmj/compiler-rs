use napi::{
  Env, Result,
  bindgen_prelude::{FnArgs, Function, JsObjectValue, Object},
};
use napi_derive::napi;

use crate::{
  ir::index::{BlockIRNode, IRDynamicInfo, IRNodeTypes},
  transform::{_enter_block, enter_block, reference},
  utils::check::is_template,
};

pub fn create_block(
  env: Env,
  node: Object<'static>,
  mut context: Object<'static>,
  is_v_for: Option<bool>,
) -> Result<(Object<'static>, Box<dyn FnOnce() -> Result<()>>)> {
  let node = wrap_fragment(env, node)?;
  context.set("node", node)?;
  let mut block = Object::new(&env)?;
  block.set("type", IRNodeTypes::BLOCK)?;
  block.set("node", node)?;
  block.set("dynamic", IRDynamicInfo::new())?;
  block.set("tempId", 0)?;
  block.set("effect", env.create_array(0))?;
  block.set("operation", env.create_array(0))?;
  block.set("returns", env.create_array(0))?;

  let (block, exit_block) = enter_block(env, context, block, is_v_for.unwrap_or(false), false)?;
  reference(context)?;
  Ok((block, exit_block))
}

#[napi(ts_return_type = "import('oxc-parser').JSXFragment")]
pub fn wrap_fragment(env: Env, node: Object) -> Result<Object> {
  if node.get_named_property::<String>("type")?.eq("JSXFragment") || is_template(Some(node)) {
    return Ok(node);
  }
  let mut obj = Object::new(&env)?;
  obj.set("type", "JSXFragment")?;
  obj.set("start", 0)?;
  obj.set("end", 0)?;
  obj.set(
    "children",
    vec![
      if node.get_named_property::<String>("type")?.eq("JSXElement") {
        node
      } else {
        let mut child = Object::new(&env)?;
        child.set("type", "JSXExpressionContainer")?;
        child.set("start", 0)?;
        child.set("end", 0)?;
        child.set("expression", node)?;
        child
      },
    ],
  )?;
  Ok(obj)
}

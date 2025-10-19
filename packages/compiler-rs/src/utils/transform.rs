use napi::{
  Env, Result,
  bindgen_prelude::{FnArgs, Function, JsObjectValue, Object},
};
use napi_derive::napi;

use crate::{
  ir::index::BlockIRNode,
  transform::{enter_block, reference},
  utils::check::is_template,
};

#[napi(ts_return_type = "[BlockIRNode, () => void]")]
pub fn create_branch(
  env: Env,
  node: Object<'static>,
  mut context: Object,
  is_v_for: Option<bool>,
) -> Result<(Object<'static>, Function<'static, (), ()>, String)> {
  let node = wrap_fragment(env, node)?;
  context.set("node", node)?;
  let branch = BlockIRNode::new(node);
  let exit_key = context.get_named_property::<i32>("exitKey")?;
  let (branch, exit_block) = context
    .get_named_property::<Function<FnArgs<(BlockIRNode, bool)>, (Object, Function<(), ()>)>>(
      "enterBlock",
    )?
    .apply(context, FnArgs::from((branch, is_v_for.unwrap_or(false))))?;
  reference(context)?;
  Ok((branch, exit_block, exit_key.to_string()))
}

pub fn _create_branch(
  env: Env,
  node: Object<'static>,
  mut context: Object<'static>,
  is_v_for: Option<bool>,
) -> Result<(Object<'static>, Box<dyn FnOnce() -> Result<()>>)> {
  let node = wrap_fragment(env, node)?;
  context.set("node", node)?;
  // let branch = BlockIRNode::new(node);
  let branch = context
    .get_named_property::<Function<Object, Object>>("createBlock")?
    .call(node)?;

  let (branch, exit_block) = enter_block(env, context, branch, is_v_for.unwrap_or(false), false)?;
  reference(context)?;
  Ok((branch, exit_block))
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

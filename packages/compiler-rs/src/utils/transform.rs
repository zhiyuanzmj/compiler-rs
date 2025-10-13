use napi::{
  Env, Result,
  bindgen_prelude::{FnArgs, Function, JsObjectValue, Object},
};
use napi_derive::napi;

use crate::{
  ir::{
    component::IRSlots,
    index::{BlockIRNode, DynamicFlag, IRDynamicInfo, IRNodeTypes},
  },
  utils::check::is_template,
};

#[napi]
pub fn new_dynamic() -> IRDynamicInfo {
  return IRDynamicInfo {
    flags: DynamicFlag::REFERENCED,
    children: Vec::new(),
    id: None,
    anchor: None,
    template: None,
    has_dynamic_child: None,
    operation: None,
  };
}

#[napi]
pub fn new_block(node: Object<'static>) -> BlockIRNode {
  BlockIRNode {
    _type: IRNodeTypes::BLOCK,
    node,
    dynamic: new_dynamic(),
    effect: Vec::new(),
    operation: Vec::new(),
    returns: Vec::new(),
    temp_id: 0,
  }
}

#[napi(ts_return_type = "[BlockIRNode, () => void]")]
pub fn create_branch(
  env: Env,
  node: Object<'static>,
  mut context: Object,
  is_v_for: Option<bool>,
) -> Result<(Object<'static>, Function<'static, (), ()>, String)> {
  let node = wrap_fragment(env, node)?;
  context.set("node", node)?;
  let branch = new_block(node);
  let exit_key = context.get_named_property::<i32>("exitKey")?;
  // let (branch, exit_block) = enter_block(env, context, branch, is_v_for.unwrap_or(false), false)?;
  let (branch, exit_block) = context
    .get_named_property::<Function<FnArgs<(BlockIRNode, bool)>, (Object, Function<(), ()>)>>(
      "enterBlock",
    )?
    .apply(context, FnArgs::from((branch, is_v_for.unwrap_or(false))))?;
  context
    .get_named_property::<Function<(), i32>>("reference")?
    .apply(context, ())?;
  Ok((branch, exit_block, exit_key.to_string()))
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

fn enter_block(
  env: Env,
  mut context: Object,
  ir: BlockIRNode,
  is_v_for: bool,
  exclude_slots: bool,
) -> Result<(BlockIRNode, impl FnOnce())> {
  let block = context.get_named_property::<BlockIRNode>("block")?;
  let template = context.get_named_property::<String>("template")?;
  let dynamic = context.get_named_property::<IRDynamicInfo>("dynamic")?;
  let children_template = context.get_named_property::<Vec<Option<String>>>("childrenTemplate")?;
  let slots = context.get_named_property::<Vec<IRSlots>>("slots")?;
  context.set("block", ir)?;
  let ir = context.get_named_property::<BlockIRNode>("block")?;
  let ir1 = context.get_named_property::<BlockIRNode>("block")?;
  context.set("dynamic", ir1.dynamic)?;
  context.set("template", String::new())?;

  context.set("childrenTemplate", env.create_array(0))?;
  if !exclude_slots {
    context.set("slots", env.create_array(0))?;
  }

  if is_v_for {
    context.set("inVFor", context.get_named_property::<i32>("inVFor")? + 1)?;
  }

  let exit_block = move || {
    context
      .get_named_property::<Function<(), ()>>("registerTemplate")
      .unwrap()
      .apply(context, ());
    context.set("block", block);
    context.set("dynamic", dynamic);
    context.set("template", template);
    context.set("childrenTemplate", children_template);
    if !exclude_slots {
      context.set("slots", slots);
    }
    if is_v_for {
      context.set(
        "inVFor",
        context.get_named_property::<i32>("inVFor").unwrap() - 1,
      );
    }
  };
  Ok((ir, exit_block))
}

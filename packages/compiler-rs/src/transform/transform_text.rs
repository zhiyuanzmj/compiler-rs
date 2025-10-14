use napi::{
  Env,
  bindgen_prelude::{Function, JsObjectValue, Object, Result},
};
use napi_derive::napi;

use crate::{
  ir::index::{DynamicFlag, IRNodeTypes, IfIRNode},
  transform::transform_node,
  utils::{check::_is_constant_node, expression::resolve_expression, transform::create_branch},
};

#[napi]
pub fn process_conditional_expression<'a>(
  env: &'a Env,
  #[napi(ts_arg_type = "import('oxc-parser').ConditionalExpression")] node: Object<'static>,
  context: Object,
) -> Result<Function<'a, (), ()>> {
  let mut dynamic = context.get_named_property::<Object>("dynamic")?;
  dynamic.set(
    "flags",
    dynamic.get_named_property::<i32>("flags")?
      | DynamicFlag::NON_TEMPLATE as i32
      | DynamicFlag::INSERT as i32,
  )?;
  let id = context
    .get_named_property::<Function<(), i32>>("reference")?
    .apply(context, ())?;
  let (.., exit_key) = create_branch(
    *env,
    node.get_named_property::<Object>("consequent")?,
    context,
    None,
  )?;
  context
    .get_named_property::<Object>("nodes")?
    .set(&exit_key, node)?;

  Ok(env.create_function_from_closure("cb", move |e| {
    let context = e.get::<Object>(0)?;
    context
      .get_named_property::<Object>("exitBlocks")?
      .get_named_property::<Function<(), ()>>(&exit_key)?
      .apply(context, ())?;
    let node = context
      .get_named_property::<Object>("nodes")?
      .get_named_property::<Object>(&exit_key)?;
    let test = node.get_named_property::<Object>("test")?;
    let alternate = node.get_named_property::<Object>("alternate")?;

    let mut dynamic = context.get_named_property::<Object>("dynamic")?;
    dynamic.set(
      "operation",
      IfIRNode {
        _type: IRNodeTypes::IF,
        id,
        positive: context
          .get_named_property::<Object>("blocks")?
          .get_named_property::<Object>(&exit_key)?,
        once: Some(
          context
            .get_named_property::<bool>("inVOnce")
            .unwrap_or(false)
            || _is_constant_node(&Some(test)),
        ),
        condition: resolve_expression(test, context),
        negative: None,
        parent: None,
        anchor: None,
      },
    )?;

    set_negative(
      *e.env,
      alternate,
      dynamic.get_named_property::<Object>("operation")?,
      context,
    )?;

    Ok(())
  })?)
}

#[napi]
pub fn process_logical_expression<'a>(
  env: &'a Env,
  node: Object<'static>,
  context: Object,
) -> Result<Function<'a, (), ()>> {
  let left = node.get_named_property::<Object>("left")?;
  let right = node.get_named_property::<Object>("right")?;
  let operator = node.get_named_property::<String>("operator")?;

  let mut dynamic = context.get_named_property::<Object>("dynamic")?;
  dynamic.set(
    "flags",
    dynamic.get_named_property::<i32>("flags")?
      | DynamicFlag::NON_TEMPLATE as i32
      | DynamicFlag::INSERT as i32,
  )?;
  let id = context
    .get_named_property::<Function<(), i32>>("reference")?
    .apply(context, ())?;
  let (.., exit_key) = create_branch(
    *env,
    if operator == "&&" { right } else { left },
    context,
    None,
  )?;
  context
    .get_named_property::<Object>("nodes")?
    .set(&exit_key, node)?;
  Ok(env.create_function_from_closure("cb", move |e| {
    let context = e.get::<Object>(0)?;
    context
      .get_named_property::<Object>("exitBlocks")?
      .get_named_property::<Function<(), ()>>(&exit_key)?
      .apply(context, ())?;
    let node = context
      .get_named_property::<Object>("nodes")?
      .get_named_property::<Object>(&exit_key)?;
    let left = node.get_named_property::<Object>("left")?;
    let right = node.get_named_property::<Object>("right")?;
    let operator = node.get_named_property::<String>("operator")?;

    let operation = IfIRNode {
      _type: IRNodeTypes::IF,
      id,
      condition: resolve_expression(left, context),
      positive: context
        .get_named_property::<Object>("blocks")?
        .get_named_property(&exit_key)?,
      once: Some(context.get_named_property::<bool>("inVOnce")? || _is_constant_node(&Some(left))),
      negative: None,
      anchor: None,
      parent: None,
    };
    context
      .get_named_property::<Object>("dynamic")?
      .set("operation", operation)?;
    set_negative(
      *e.env,
      if operator == "&&" { left } else { right },
      context
        .get_named_property::<Object>("dynamic")?
        .get_named_property::<Object>("operation")?,
      context,
    )?;
    Ok(())
  })?)
}

pub fn set_negative(
  env: Env,
  node: Object<'static>,
  mut operation: Object,
  context: Object,
) -> Result<()> {
  let node_type = node.get_named_property::<String>("type")?;
  if node_type == "ConditionalExpression" {
    let (branch, on_exit, _) = create_branch(
      env,
      node.get_named_property::<Object>("consequent")?,
      context,
      None,
    )?;
    let test = node.get_named_property::<Object>("test")?;
    let negative = IfIRNode {
      _type: IRNodeTypes::IF,
      id: -1,
      condition: resolve_expression(test, context),
      positive: branch,
      once: Some(context.get_named_property::<bool>("inVOnce")? || _is_constant_node(&Some(test))),
      negative: None,
      anchor: None,
      parent: None,
    };
    operation.set("negative", negative)?;
    transform_node(context)?;
    set_negative(
      env,
      node.get_named_property::<Object>("alternate")?,
      operation.get_named_property::<Object>("negative")?,
      context,
    )?;
    on_exit.call(())?;
  } else if node_type == "LogicalExpression" {
    let left = node.get_named_property::<Object>("left")?;
    let right = node.get_named_property::<Object>("right")?;
    let operator = node.get_named_property::<String>("operator")?;
    let (branch, on_exit, ..) = create_branch(
      env,
      if operator.eq("&&") { right } else { left },
      context,
      None,
    )?;
    let negative = IfIRNode {
      _type: IRNodeTypes::IF,
      id: -1,
      condition: resolve_expression(left, context),
      positive: branch,
      once: Some(context.get_named_property::<bool>("inVOnce")? || _is_constant_node(&Some(left))),
      negative: None,
      anchor: None,
      parent: None,
    };
    operation.set("negative", negative)?;
    transform_node(context)?;
    set_negative(
      env,
      if operator.eq("&&") { left } else { right },
      operation.get_named_property::<Object>("negative")?,
      context,
    )?;
    on_exit.call(())?;
  } else {
    let (branch, on_exit, ..) = create_branch(env, node, context, None)?;
    operation.set("negative", branch)?;
    transform_node(context)?;
    on_exit.call(())?;
  }
  Ok(())
}

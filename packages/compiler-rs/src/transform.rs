use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  rc::{Rc, Weak},
};

use napi::{
  Env, Result,
  bindgen_prelude::{FnArgs, Function, JsObjectValue, Object},
};
use napi_derive::napi;
pub mod transform_children;
pub mod transform_element;
pub mod transform_template_ref;
pub mod transform_text;
pub mod v_bind;
pub mod v_for;
pub mod v_html;
pub mod v_if;
pub mod v_model;
pub mod v_on;
pub mod v_once;
pub mod v_show;
pub mod v_slot;
pub mod v_slots;
pub mod v_text;

use crate::{
  ir::{
    component::IRSlots,
    index::{
      _BlockIRNode, BlockIRNode, DynamicFlag, IRDynamicInfo, IREffect, IRNodeTypes, Modifiers,
      OperationNode, RootIRNode, SimpleExpressionNode,
    },
  },
  transform::{
    transform_children::transform_children, transform_element::transform_element,
    transform_template_ref::transform_template_ref, transform_text::transform_text,
    v_for::transform_v_for, v_if::transform_v_if, v_once::transform_v_once,
    v_slot::transform_v_slot,
  },
  utils::{check::is_constant_node, error::CompilerError, expression::_is_constant_expression},
};

// #[napi(object)]
pub struct TransformOptions {
  pub source: String,
  pub templates: Vec<String>,
  /**
   * Whether to compile components to createComponentWithFallback.
   * @default false
   */
  pub with_fallback: bool,
  /**
   * Indicates that transforms and codegen should try to output valid TS code
   */
  pub is_ts: bool,
  /**
   * Separate option for end users to extend the native elements list
   */
  pub is_custom_element: Box<dyn Fn(String) -> bool>,
  pub on_error: Box<dyn Fn(CompilerError) -> ()>,
  /**
   * Generate source map?
   * @default false
   */
  pub source_map: bool,
  /**
   * Filename for source map generation.
   * Also used for self-recursive reference in templates
   * @default 'index.jsx'
   */
  pub filename: String,
}

impl TransformOptions {
  pub fn new(env: Env) -> Self {
    TransformOptions {
      source: String::new(),
      source_map: false,
      filename: "index.jsx".to_string(),
      templates: Vec::new(),
      is_custom_element: Box::new(|_| false),
      is_ts: false,
      with_fallback: false,
      on_error: Box::new(move |error: CompilerError| env.throw(error).unwrap()),
    }
  }
}

#[napi(object)]
pub struct DirectiveTransformResult {
  pub key: SimpleExpressionNode,
  pub value: SimpleExpressionNode,
  #[napi(ts_type = "'.' | '^'")]
  pub modifier: Option<String>,
  pub runtime_camelize: Option<bool>,
  pub handler: Option<bool>,
  pub handler_modifiers: Option<Modifiers>,
  pub model: Option<bool>,
  pub model_modifiers: Option<Vec<String>>,
}

impl DirectiveTransformResult {
  pub fn new(key: SimpleExpressionNode, value: SimpleExpressionNode) -> Self {
    DirectiveTransformResult {
      key,
      value,
      modifier: None,
      runtime_camelize: None,
      handler: None,
      handler_modifiers: None,
      model: None,
      model_modifiers: None,
    }
  }
}

pub fn is_operation(expressions: Vec<&SimpleExpressionNode>, context: &Object) -> bool {
  if context
    .get_named_property::<bool>("inVOnce")
    .ok()
    .is_some_and(|a| a == true)
  {
    return true;
  }
  let expressions: Vec<&SimpleExpressionNode> = expressions
    .into_iter()
    .filter(|exp| !_is_constant_expression(exp))
    .collect();
  if expressions.len() == 0 {
    return true;
  }
  expressions.into_iter().all(|exp| is_constant_node(exp.ast))
}

pub struct TransformContext {
  parent: RefCell<Weak<TransformContext>>,
  root: RefCell<Weak<TransformContext>>,
  index: i32,

  block: Rc<RefCell<_BlockIRNode>>,
  options: TransformOptions,

  template: String,
  children_template: Vec<String>,
  in_v_once: bool,
  in_v_for: i32,

  slots: Vec<IRSlots>,

  global_id: i32,

  ir: RootIRNode,
  node: Object<'static>,
}

impl TransformContext {
  pub fn new(ir: RootIRNode, node: Object<'static>, options: TransformOptions) -> Rc<Self> {
    let block = ir.block.borrow_mut().upgrade().unwrap();
    let context = Rc::new(TransformContext {
      parent: RefCell::new(Weak::new()),
      root: RefCell::new(Weak::new()),
      index: 0,
      template: String::new(),
      children_template: Vec::new(),
      in_v_once: false,
      in_v_for: 0,
      slots: Vec::new(),
      global_id: 0,
      node,
      ir,
      block,
      options,
    });
    *context.root.borrow_mut() = Rc::downgrade(&context);
    context
  }
}

#[napi]
pub fn reference(context: Object) -> Result<i32> {
  let mut dynamic = context
    .get_named_property::<Object>("block")?
    .get_named_property::<Object>("dynamic")?;
  if let Ok(id) = dynamic.get_named_property::<i32>("id") {
    return Ok(id);
  }
  dynamic.set(
    "flags",
    dynamic.get_named_property::<i32>("flags")? | DynamicFlag::REFERENCED as i32,
  )?;
  let id = increase_id(context)?;
  dynamic.set("id", id)?;
  Ok(id)
}

pub fn register_effect(
  context: &Object,
  is_operation: bool,
  operation: OperationNode,
  get_effect_index: Option<Rc<RefCell<Box<dyn FnMut() -> i32>>>>,
  get_operation_index: Option<Rc<RefCell<Box<dyn FnMut() -> i32>>>>,
) -> Result<()> {
  if is_operation {
    register_operation(context, operation, get_operation_index)?;
    return Ok(());
  }

  let effects = context
    .get_named_property::<Object>("block")?
    .get_named_property::<Object>("effect")?;
  let get_effect_index: Rc<RefCell<Box<dyn FnMut() -> i32>>> =
    if let Some(get_effect_index) = get_effect_index {
      get_effect_index
    } else {
      Rc::new(RefCell::new(Box::new(move || {
        effects.get_named_property::<i32>("length").unwrap()
      })))
    };
  effects
    .get_named_property::<Function<FnArgs<(i32, i32, IREffect)>, Object>>("splice")?
    .apply(
      effects,
      FnArgs::from((
        get_effect_index.borrow_mut()(),
        0,
        IREffect {
          expressions: vec![],
          operations: vec![operation],
        },
      )),
    )?;
  Ok(())
}

pub fn register_operation(
  context: &Object,
  operation: OperationNode,
  get_operation_index: Option<Rc<RefCell<Box<dyn FnMut() -> i32>>>>,
) -> Result<()> {
  let operations = context
    .get_named_property::<Object>("block")?
    .get_named_property::<Object>("operation")?;
  let get_operation_index: Rc<RefCell<Box<dyn FnMut() -> i32>>> =
    if let Some(get_operation_index) = get_operation_index {
      get_operation_index
    } else {
      Rc::new(RefCell::new(Box::new(move || {
        operations.get_named_property::<i32>("length").unwrap()
      })))
    };
  operations
    .get_named_property::<Function<FnArgs<(i32, i32, OperationNode)>, Object>>("splice")?
    .apply(
      operations,
      FnArgs::from((get_operation_index.borrow_mut()(), 0, operation)),
    )?;
  Ok(())
}

#[napi]
pub fn push_template(context: Object, content: String) -> Result<i32> {
  let templates = context
    .get_named_property::<Object>("ir")?
    .get_named_property::<Vec<String>>("templates")?;
  if let Some(existing) = templates.iter().position(|i| i == &content) {
    return Ok(existing as i32);
  }
  let len = templates.len();
  let templates = context
    .get_named_property::<Object>("ir")?
    .get_named_property::<Object>("templates")?;
  templates
    .get_named_property::<Function<String, i32>>("push")?
    .apply(templates, content)?;
  Ok(len as i32)
}

#[napi]
pub fn register_template(context: Object) -> Result<i32> {
  let template = context.get_named_property::<String>("template")?;
  if template.is_empty() {
    return Ok(-1);
  }
  let id = push_template(context, template)?;
  context
    .get_named_property::<Object>("block")?
    .get_named_property::<Object>("dynamic")?
    .set("template", id)?;
  Ok(id)
}

pub fn enter_block(
  env: Env,
  mut this: Object<'static>,
  ir: Object<'static>,
  is_v_for: bool,
  exclude_slots: bool,
) -> Result<(Object<'static>, Box<dyn FnOnce() -> Result<()>>)> {
  let block = this.get_named_property::<Object>("block")?;
  let template = this.get_named_property::<String>("template")?;
  let children_template = this.get_named_property::<Vec<String>>("childrenTemplate")?;
  let slots = this.get_named_property::<Vec<Object>>("slots")?;

  this.set("block", ir)?;
  this.set("template", String::new())?;
  this.set("childrenTemplate", env.create_array(0))?;
  if !exclude_slots {
    this.set("slots", env.create_array(0))?;
  }

  if is_v_for {
    this.set("inVFor", this.get_named_property::<i32>("inVFor")? + 1)?;
  }

  let exit_block = Box::new(move || {
    // exit
    register_template(this)?;
    this.set("block", block)?;
    this.set("template", template)?;
    this.set("childrenTemplate", children_template)?;
    if !exclude_slots {
      this.set("slots", slots)?;
    }
    if is_v_for {
      this.set(
        "inVFor",
        this.get_named_property::<i32>("inVFor").unwrap() + 1,
      )?;
    }
    Ok(())
  }) as Box<dyn FnOnce() -> Result<()>>;

  Ok((ir, exit_block))
}

pub fn _enter_block(
  env: Env,
  mut this: Object<'static>,
  ir: BlockIRNode,
  is_v_for: bool,
  exclude_slots: bool,
) -> Result<(BlockIRNode, Box<dyn FnOnce() -> Result<()>>)> {
  let block = this.get_named_property::<Object>("block")?;
  let template = this.get_named_property::<String>("template")?;
  let children_template = this.get_named_property::<Vec<String>>("childrenTemplate")?;
  let slots = this.get_named_property::<Vec<Object>>("slots")?;

  // this.set("block", ir)?;
  this.set("template", String::new())?;
  this.set("childrenTemplate", env.create_array(0))?;
  if !exclude_slots {
    this.set("slots", env.create_array(0))?;
  }

  if is_v_for {
    this.set("inVFor", this.get_named_property::<i32>("inVFor")? + 1)?;
  }

  let exit_block = Box::new(move || {
    // exit
    register_template(this)?;
    this.set("block", block)?;
    this.set("template", template)?;
    this.set("childrenTemplate", children_template)?;
    if !exclude_slots {
      this.set("slots", slots)?;
    }
    if is_v_for {
      this.set(
        "inVFor",
        this.get_named_property::<i32>("inVFor").unwrap() + 1,
      )?;
    }
    Ok(())
  }) as Box<dyn FnOnce() -> Result<()>>;

  Ok((ir, exit_block))
}

pub fn create<'a>(env: Env, context: Object, node: Object, index: i32) -> Result<Object<'a>> {
  context
    .get_named_property::<Object>("block")?
    .set("dynamic", IRDynamicInfo::new())?;
  let mut object = Object::new(&env)?;
  object.set("block", context.get_named_property::<Object>("block"))?;
  object.set("inVFor", context.get_named_property::<i32>("inVFor"))?;
  object.set("inVOnce", context.get_named_property::<bool>("inVOnce"))?;
  object.set("ir", context.get_named_property::<Object>("ir"))?;
  object.set("options", context.get_named_property::<Object>("options"))?;
  object.set("root", context.get_named_property::<Object>("root"))?;
  object.set("seen", context.get_named_property::<HashSet<i32>>("seen"))?;
  object.set("slots", context.get_named_property::<Object>("slots"))?;

  object.set("node", node)?;
  object.set("parent", context)?;
  object.set("index", index)?;

  object.set("template", String::new())?;
  object.set("childrenTemplate", env.create_array(0))?;
  Ok(object)
}

pub fn increase_id(context: Object) -> Result<i32> {
  let mut root: Object = context.get_named_property("root")?;
  let current = root.get_named_property("globalId")?;
  root.set("globalId", current + 1)?;
  Ok(current)
}

#[napi]
pub fn transform_node(env: Env, mut context: Object<'static>) -> Result<()> {
  let mut node = context.get_named_property::<Object>("node")?;

  let mut exit_fns = vec![];
  for node_transform in vec![
    transform_v_once,
    transform_v_if,
    transform_v_for,
    transform_template_ref,
    transform_element,
    transform_text,
    transform_v_slot,
    transform_children,
  ] {
    let on_exit = node_transform(env, node, context)?;
    if let Some(on_exit) = on_exit {
      exit_fns.push(on_exit);
    }
    // node may have been replaced
    node = context.get_named_property::<Object>("node")?;
  }

  context.set("node", node)?;
  let mut i = exit_fns.len();
  while i > 0 {
    i = i - 1;
    let on_exit = exit_fns.pop().unwrap();
    on_exit()?;
  }

  if node.get_named_property::<String>("type")?.eq("ROOT") {
    register_template(context)?;
  }
  Ok(())
}

#[napi]
pub fn new_block(node: Object<'static>) -> BlockIRNode {
  BlockIRNode::new(node)
}

use std::{
  cell::{RefCell, RefMut},
  collections::HashSet,
  rc::{Rc, Weak},
};

use napi::{
  Result,
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
      BlockIRNode, IRDynamicInfo, IREffect, Modifiers, OperationNode, RootIRNode,
      SimpleExpressionNode,
    },
  },
  utils::{check::is_constant_node, expression::_is_constant_expression},
};

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

// #[napi]
pub struct TransformContext<T> {
  parent: RefCell<Weak<TransformContext<T>>>,
  root: RefCell<Weak<TransformContext<T>>>,
  index: i32,

  ir: RefCell<RootIRNode>,
  component: HashSet<String>,
  directive: HashSet<String>,
  slots: Vec<IRSlots>,
  global_id: i32,
  in_v_once: bool,
  in_v_for: i32,
  children_template: Vec<String>,
  template: String,
  node: T,
}

impl<T> TransformContext<T> {
  // #[napi(constructor)]
  pub fn new(ir: RootIRNode, node: T) -> Rc<Self> {
    let ir = RefCell::new(ir);
    let context = Rc::new(TransformContext {
      parent: RefCell::new(Weak::new()),
      root: RefCell::new(Weak::new()),
      index: 0,
      // options,
      template: String::new(),
      children_template: Vec::new(),
      in_v_once: false,
      in_v_for: 0,
      component: HashSet::new(),
      directive: HashSet::new(),
      slots: Vec::new(),
      global_id: 0,
      ir,
      node,
    });
    *context.root.borrow_mut() = Rc::downgrade(&context);
    context
  }
  pub fn block<'b>(&'b self) -> RefMut<'b, BlockIRNode> {
    RefMut::map(self.ir.borrow_mut(), |ir| &mut ir.block)
  }
  pub fn dynamic<'c>(&'c self) -> RefMut<'c, IRDynamicInfo> {
    RefMut::map(self.block(), |block| &mut block.dynamic)
  }
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
pub fn transform_node(mut context: Object) -> Result<()> {
  let mut node = context.get_named_property::<Object>("node")?;

  // apply transform plugins
  let node_transforms = context
    .get_named_property::<Object>("options")?
    .get_named_property::<Vec<Function<FnArgs<(Object, Object)>, Option<Function<Object, ()>>>>>(
      "nodeTransforms",
    )?;
  let mut exit_fns = vec![];
  for node_transform in node_transforms {
    let on_exit = node_transform.call(FnArgs::from((node, context)))?;
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
    exit_fns[i].apply(context, context)?;
  }

  if node.get_named_property::<String>("type")?.eq("ROOT") {
    context
      .get_named_property::<Function<(), i32>>("registerTemplate")?
      .apply(context, ())?;
  }
  Ok(())
}

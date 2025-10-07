use std::{
  cell::{RefCell, RefMut},
  collections::HashSet,
  rc::{Rc, Weak},
};

use napi::bindgen_prelude::{JsObjectValue, Object};
use napi_derive::napi;
pub mod v_bind;
pub mod v_html;
pub mod v_once;
pub mod v_show;
pub mod v_slots;
pub mod v_text;

use crate::{
  ir::{
    component::IRSlots,
    index::{BlockIRNode, IRDynamicInfo, Modifiers, RootIRNode, SimpleExpressionNode},
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

//#[napi]
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

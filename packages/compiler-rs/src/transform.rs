use std::{
  cell::{RefCell, RefMut},
  collections::HashSet,
  rc::{Rc, Weak},
};

use napi::{
  Either, Env, Result,
  bindgen_prelude::{FnArgs, Function, JsObjectValue, Object},
};
use napi_derive::napi;
pub mod transform_template_ref;
pub mod v_bind;
pub mod v_for;
pub mod v_html;
pub mod v_once;
pub mod v_show;
pub mod v_slot;
pub mod v_slots;
pub mod v_text;

use crate::{
  ir::{
    component::IRSlots,
    index::{BlockIRNode, IRDynamicInfo, Modifiers, RootIRNode, SimpleExpressionNode},
  },
  utils::{
    check::{is_constant_node, is_template},
    expression::_is_constant_expression,
    text::get_text,
    utils::find_prop,
  },
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

#[napi]
pub fn create_structural_directive_transform<'a>(
  env: &'a Env,
  name: Either<String, Vec<String>>,
  #[napi(
    ts_arg_type = "(node: import('oxc-parser').JSXElement, dir: import('oxc-parser').JSXAttribute, context: object) => void | (() => void)"
  )]
  _fn: Function<(Object, Object), ()>,
) -> Result<Function<'a, (), ()>> {
  let matches = |n: String| match name {
    Either::A(name) => name == n,
    Either::B(names) => names.contains(&n),
  };

  Ok(env.create_function_from_closure("cb", move |e| {
    let node = e.get::<Object>(0)?;
    let context = e.get::<Object>(1)?;

    if node.get_named_property::<String>("type")?.eq("JSXElement") {
      // structural directive transforms are not concerned with slots
      // as they are handled separately in vSlot.ts
      if is_template(Some(node)) && find_prop(node, Either::A(String::from("v-slot"))).is_some() {
        return Ok(());
      }
      let attributes = node
        .get_named_property::<Object>("openingElement")?
        .get_named_property::<Vec<Object>>("attributes")?;
      // let mut exit_fns = Vec::new();
      for prop in attributes {
        if prop
          .get_named_property::<String>("type")?
          .eq("JSXAttribute")
        {
          continue;
        }
        let prop_name = get_text(prop.get_named_property::<Object>("name")?, context);
        // if prop_name.starts_with("v-") && matches(prop_name[2..].to_string()) {
        // attributes.splice
        // }
      }
    }

    Ok(())
  })?)
}

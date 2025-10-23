use std::{
  cell::RefCell,
  collections::HashSet,
  mem,
  rc::{Rc, Weak},
};

use napi::{
  Env, Result,
  bindgen_prelude::{Function, JsObjectValue, Object},
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
      BlockIRNode, DynamicFlag, IRDynamicInfo, IREffect, Modifiers, OperationNode, RootIRNode,
      SimpleExpressionNode,
    },
  },
  transform::{
    transform_children::transform_children, transform_element::transform_element,
    transform_template_ref::transform_template_ref, transform_text::transform_text,
    v_for::transform_v_for, v_if::transform_v_if, v_once::transform_v_once,
    v_slot::transform_v_slot,
  },
  utils::{
    check::{is_constant_node, is_template},
    error::CompilerError,
    expression::_is_constant_expression,
  },
};

#[napi(object)]
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
  pub is_custom_element: Function<'static, String, bool>,
  pub on_error: Function<'static, Object<'static>, ()>,
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
  pub fn new(env: &'static Env, source: String) -> Self {
    TransformOptions {
      source,
      source_map: false,
      filename: "index.tsx".to_string(),
      templates: Vec::new(),
      is_ts: true,
      with_fallback: false,
      is_custom_element: env
        .create_function_from_closure("cb", |_| Ok(false))
        .unwrap(),
      on_error: env
        .create_function_from_closure("cb", |e| {
          let error = e.get::<CompilerError>(0)?;
          env.throw(error)?;
          Ok(())
        })
        .unwrap(),
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

pub struct TransformContext {
  pub env: Env,
  pub parent: RefCell<Weak<TransformContext>>,
  pub root: RefCell<Weak<TransformContext>>,
  pub index: i32,

  pub block: RefCell<BlockIRNode>,
  pub options: Rc<TransformOptions>,

  pub template: RefCell<String>,
  pub children_template: RefCell<Vec<String>>,

  pub in_v_once: RefCell<bool>,
  pub in_v_for: RefCell<i32>,

  pub slots: Rc<RefCell<Vec<IRSlots>>>,

  pub seen: Rc<RefCell<HashSet<i32>>>,

  global_id: RefCell<i32>,

  pub ir: Rc<RefCell<RootIRNode>>,
  pub node: RefCell<Object<'static>>,
}

impl TransformContext {
  pub fn new(
    env: Env,
    mut ir: RootIRNode,
    node: Object<'static>,
    options: TransformOptions,
  ) -> Rc<Self> {
    let block = mem::take(&mut ir.block);
    let context = Rc::new(TransformContext {
      env,
      parent: RefCell::new(Weak::new()),
      root: RefCell::new(Weak::new()),
      index: 0,
      template: RefCell::new(String::new()),
      children_template: RefCell::new(Vec::new()),
      in_v_once: RefCell::new(false),
      in_v_for: RefCell::new(0),
      slots: Rc::new(RefCell::new(Vec::new())),
      seen: Rc::new(RefCell::new(HashSet::new())),
      global_id: RefCell::new(0),
      node: RefCell::new(node),
      ir: Rc::new(RefCell::new(ir)),
      block: RefCell::new(block),
      options: Rc::new(options),
    });
    *context.root.borrow_mut() = Rc::downgrade(&context);
    context
  }

  pub fn increase_id(self: &Rc<Self>) -> Result<i32> {
    let root = self.root.borrow_mut().upgrade().unwrap();
    let current = *root.global_id.borrow();
    *root.global_id.borrow_mut() += 1;
    Ok(current)
  }

  pub fn reference(self: &Rc<Self>, dynamic: &mut IRDynamicInfo) -> Result<i32> {
    if let Some(id) = dynamic.id {
      return Ok(id);
    }
    dynamic.flags = dynamic.flags | DynamicFlag::REFERENCED as i32;
    let id = self.increase_id()?;
    dynamic.id = Some(id);
    Ok(id)
  }

  pub fn is_operation(self: &Rc<Self>, expressions: Vec<&SimpleExpressionNode>) -> bool {
    if self.in_v_once.borrow().eq(&true) {
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

  pub fn register_effect<'a>(
    self: &'a Rc<Self>,
    context_block: &mut BlockIRNode,
    is_operation: bool,
    operation: OperationNode,
    get_effect_index: Option<Rc<RefCell<Box<dyn FnMut() -> i32 + 'a>>>>,
    get_operation_index: Option<Rc<RefCell<Box<dyn FnMut() -> i32 + 'a>>>>,
  ) -> Result<()> {
    if is_operation {
      return self.register_operation(context_block, operation, get_operation_index);
    }

    let index = if let Some(get_effect_index) = get_effect_index {
      get_effect_index.borrow_mut()() as usize
    } else {
      context_block.effect.len()
    };
    context_block.effect.splice(
      index..index,
      vec![IREffect {
        expressions: vec![],
        operations: vec![operation],
      }],
    );
    Ok(())
  }

  pub fn register_operation<'a>(
    self: &'a Rc<Self>,
    context_block: &mut BlockIRNode,
    operation: OperationNode,
    get_operation_index: Option<Rc<RefCell<Box<dyn FnMut() -> i32 + 'a>>>>,
  ) -> Result<()> {
    let index = if let Some(get_operation_index) = get_operation_index {
      get_operation_index.borrow_mut()() as usize
    } else {
      context_block.operation.len()
    };
    context_block
      .operation
      .splice(index..index, vec![operation]);

    Ok(())
  }

  pub fn push_template(self: &Rc<Self>, content: String) -> Result<i32> {
    let templates = &mut self.ir.borrow_mut().templates;
    if let Some(existing) = templates.iter().position(|i| i == &content) {
      return Ok(existing as i32);
    }
    let len = templates.len();
    templates.push(content);
    Ok(len as i32)
  }

  pub fn register_template(self: &Rc<Self>, dynamic: &mut IRDynamicInfo) -> Result<i32> {
    let template = self.template.borrow();
    if template.is_empty() {
      return Ok(-1);
    }
    let id = self.push_template(template.clone())?;
    dynamic.template = Some(id);
    Ok(id)
  }

  pub fn enter_block<'a>(
    self: &'a Rc<Self>,
    context_block: &'a mut BlockIRNode,
    ir: BlockIRNode,
    is_v_for: bool,
    exclude_slots: bool,
  ) -> Result<Box<dyn FnOnce() -> Result<BlockIRNode> + 'a>> {
    let block = mem::take(&mut *context_block);
    let template = mem::take(&mut *self.template.borrow_mut());
    let children_template = mem::take(&mut *self.children_template.borrow_mut());
    let mut slots = None;

    *context_block = ir;
    if !exclude_slots {
      slots = Some(mem::take(&mut *self.slots.borrow_mut()));
    }

    if is_v_for {
      *self.in_v_for.borrow_mut() += 1;
    }

    let exit_block = Box::new(move || {
      // exit
      self.register_template(&mut context_block.dynamic)?;
      let return_block = mem::take(context_block);
      *context_block = block;
      *self.template.borrow_mut() = template;
      *self.children_template.borrow_mut() = children_template;
      if !exclude_slots && let Some(slots) = slots {
        *self.slots.borrow_mut() = slots;
      }
      if is_v_for {
        *self.in_v_for.borrow_mut() -= 1;
      }
      Ok(return_block)
    }) as Box<dyn FnOnce() -> Result<BlockIRNode>>;

    Ok(exit_block)
  }

  pub fn create_block<'a>(
    self: &'a Rc<Self>,
    context_block: &'a mut BlockIRNode,
    node: Object<'static>,
    is_v_for: Option<bool>,
  ) -> Result<Box<dyn FnOnce() -> Result<BlockIRNode> + 'a>> {
    // wrap_fragment
    let node = if node.get_named_property::<String>("type")?.eq("JSXFragment") || is_template(&node)
    {
      node
    } else {
      let mut obj = Object::new(&self.env)?;
      obj.set("type", "JSXFragment")?;
      obj.set("start", 0)?;
      obj.set("end", 0)?;
      obj.set(
        "children",
        vec![
          if node.get_named_property::<String>("type")?.eq("JSXElement") {
            node
          } else {
            let mut child = Object::new(&self.env)?;
            child.set("type", "JSXExpressionContainer")?;
            child.set("start", 0)?;
            child.set("end", 0)?;
            child.set("expression", node)?;
            child
          },
        ],
      )?;
      obj
    };

    *self.node.borrow_mut() = node;
    let block = BlockIRNode::new(Some(node));
    let _context_block = context_block as *mut BlockIRNode;
    let exit_block = self.enter_block(
      unsafe { &mut *_context_block },
      block,
      is_v_for.unwrap_or(false),
      false,
    )?;
    self.reference(&mut context_block.dynamic)?;
    Ok(exit_block)
  }

  pub fn create(
    self: &Rc<Self>,
    node: Object<'static>,
    index: i32,
    context_block: &mut BlockIRNode,
  ) -> Rc<Self> {
    let mut block = mem::take(context_block);
    block.dynamic = IRDynamicInfo::new();

    Rc::new(TransformContext {
      env: self.env,
      block: RefCell::new(block),
      in_v_for: RefCell::new(*self.in_v_for.borrow()),
      in_v_once: RefCell::new(*self.in_v_once.borrow()),
      ir: Rc::clone(&self.ir),
      global_id: RefCell::new(0),
      options: Rc::clone(&self.options),
      root: RefCell::new(Rc::downgrade(&self.root.borrow().upgrade().unwrap())),
      seen: Rc::clone(&self.seen),
      slots: Rc::clone(&self.slots),
      parent: RefCell::new(Rc::downgrade(&self)),
      node: RefCell::new(node),
      index,
      template: RefCell::new(String::new()),
      children_template: RefCell::new(vec![]),
    })
  }

  pub fn transform_node<'a>(
    self: &'a Rc<TransformContext>,
    context_block: &'a mut BlockIRNode,
    parent_dynamic: &'a mut IRDynamicInfo,
  ) -> Result<()> {
    // let mut node = mem::take(&mut self.node.borrow_mut());
    let mut node = *self.node.borrow_mut();

    let block = context_block as *mut BlockIRNode;
    let parent_dynamic = parent_dynamic as *mut IRDynamicInfo;
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
      let on_exit = node_transform(node, self, unsafe { &mut *block }, unsafe {
        &mut *parent_dynamic
      })?;
      if let Some(on_exit) = on_exit {
        exit_fns.push(on_exit);
      }
      // node may have been replaced
      if let Ok(node_mut) = self.node.try_borrow_mut() {
        node = *node_mut;
      };
    }

    *self.node.borrow_mut() = node;
    let mut i = exit_fns.len();
    while i > 0 {
      i = i - 1;
      let on_exit = exit_fns.pop().unwrap();
      on_exit()?;
    }

    if node.get_named_property::<String>("type")?.eq("ROOT") {
      self.register_template(&mut context_block.dynamic)?;
    }
    Ok(())
  }
}

#[napi]
pub fn transform(env: Env, node: Object<'static>, options: TransformOptions) -> Result<RootIRNode> {
  let templates = options.templates.clone();
  let source = options.source.clone();
  let ir = RootIRNode::new(node, source.clone(), templates);

  let context = TransformContext::new(env, ir, node, options);

  // let mut block = context.block.borrow_mut();
  let mut block = context.block.take();
  context.transform_node(&mut block, &mut IRDynamicInfo::new())?;
  // let block = *block;

  let mut ir = mem::replace(
    &mut *context.ir.borrow_mut(),
    RootIRNode::new(node, String::new(), vec![]),
  );
  ir.block = block;
  Ok(ir)
}

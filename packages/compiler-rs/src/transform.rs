use std::{cell::RefCell, collections::HashSet, mem, rc::Rc};

use napi::Either;
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::{
  Expression, JSXChild, JSXClosingFragment, JSXExpressionContainer, JSXFragment, JSXOpeningFragment,
};
use oxc_span::Span;
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
mod v_once;
pub mod v_show;
pub mod v_slot;
pub mod v_slots;
pub mod v_text;

use crate::{
  generate::{CodegenOptions, VaporCodegenResult, generate},
  ir::{
    component::IRSlots,
    index::{
      BlockIRNode, DynamicFlag, IRDynamicInfo, IREffect, Modifiers, OperationNode, RootIRNode,
      RootNode, SimpleExpressionNode,
    },
  },
  transform::{
    transform_children::transform_children, transform_element::transform_element,
    transform_template_ref::transform_template_ref, transform_text::transform_text,
    v_for::transform_v_for, v_if::transform_v_if, v_once::transform_v_once,
    v_slot::transform_v_slot, v_slots::transform_v_slots,
  },
  utils::{
    check::{is_constant_node, is_template},
    error::ErrorCodes,
    expression::to_jsx_expression,
  },
};

pub struct TransformOptions {
  pub source: String,
  pub templates: Vec<String>,
  pub with_fallback: bool,
  pub is_custom_element: Box<dyn Fn(String) -> bool>,
  pub on_error: Box<dyn Fn(ErrorCodes)>,
  pub source_map: bool,
  pub filename: String,
}
impl TransformOptions {
  pub fn build(source: String) -> Self {
    TransformOptions {
      source,
      filename: String::from("index.jsx"),
      templates: vec![],
      source_map: false,
      with_fallback: false,
      is_custom_element: Box::new(|_| false),
      on_error: Box::new(|_| {}),
    }
  }
}

pub struct DirectiveTransformResult<'a> {
  pub key: SimpleExpressionNode<'a>,
  pub value: SimpleExpressionNode<'a>,
  pub modifier: Option<String>,
  pub runtime_camelize: Option<bool>,
  pub handler: Option<bool>,
  pub handler_modifiers: Option<Modifiers>,
  pub model: Option<bool>,
  pub model_modifiers: Option<Vec<String>>,
}

impl<'a> DirectiveTransformResult<'a> {
  pub fn new(key: SimpleExpressionNode<'a>, value: SimpleExpressionNode<'a>) -> Self {
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

pub type ContextNode<'a> = Either<RootNode<'a>, JSXChild<'a>>;

pub struct TransformContext<'a> {
  pub allocator: &'a Allocator,
  pub index: RefCell<i32>,

  pub block: RefCell<BlockIRNode<'a>>,
  pub options: TransformOptions,

  pub template: RefCell<String>,
  pub children_template: RefCell<Vec<String>>,

  pub in_v_once: RefCell<bool>,
  pub in_v_for: RefCell<i32>,

  pub slots: RefCell<Vec<IRSlots<'a>>>,

  pub seen: Rc<RefCell<HashSet<u32>>>,

  global_id: RefCell<i32>,

  pub ir: Rc<RefCell<RootIRNode<'a>>>,
  pub node: RefCell<ContextNode<'a>>,
  pub parent_node: RefCell<Option<ContextNode<'a>>>,

  pub parent_dynamic: RefCell<IRDynamicInfo<'a>>,
}

impl<'a> TransformContext<'a> {
  pub fn new(
    allocator: &'a Allocator,
    mut ir: RootIRNode<'a>,
    node: ContextNode<'a>,
    options: TransformOptions,
  ) -> Self {
    let block = mem::take(&mut ir.block);
    let context = TransformContext {
      allocator,
      index: RefCell::new(0),
      template: RefCell::new(String::new()),
      children_template: RefCell::new(Vec::new()),
      in_v_once: RefCell::new(false),
      in_v_for: RefCell::new(0),
      slots: RefCell::new(Vec::new()),
      seen: Rc::new(RefCell::new(HashSet::new())),
      global_id: RefCell::new(0),
      node: RefCell::new(node),
      parent_node: RefCell::new(None),
      parent_dynamic: RefCell::new(IRDynamicInfo::new()),
      ir: Rc::new(RefCell::new(ir)),
      block: RefCell::new(block),
      options,
    };
    context
  }

  pub fn increase_id(self: &Self) -> i32 {
    let current = *self.global_id.borrow();
    *self.global_id.borrow_mut() += 1;
    current
  }

  pub fn reference(self: &Self, dynamic: &mut IRDynamicInfo) -> i32 {
    if let Some(id) = dynamic.id {
      return id;
    }
    dynamic.flags = dynamic.flags | DynamicFlag::Referenced as i32;
    let id = self.increase_id();
    dynamic.id = Some(id);
    id
  }

  pub fn is_operation(self: &Self, expressions: Vec<&SimpleExpressionNode>) -> bool {
    if self.in_v_once.borrow().eq(&true) {
      return true;
    }
    let expressions: Vec<&SimpleExpressionNode> = expressions
      .into_iter()
      .filter(|exp| !exp.is_constant_expression())
      .collect();
    if expressions.len() == 0 {
      return true;
    }
    expressions
      .iter()
      .all(|exp| is_constant_node(&exp.ast.as_ref()))
  }

  pub fn register_effect(
    self: &'a Self,
    context_block: &mut BlockIRNode<'a>,
    is_operation: bool,
    operation: OperationNode<'a>,
    get_effect_index: Option<Rc<RefCell<Box<dyn FnMut() -> i32 + 'a>>>>,
    get_operation_index: Option<Rc<RefCell<Box<dyn FnMut() -> i32 + 'a>>>>,
  ) {
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
  }

  pub fn register_operation(
    self: &Self,
    context_block: &mut BlockIRNode<'a>,
    operation: OperationNode<'a>,
    get_operation_index: Option<Rc<RefCell<Box<dyn FnMut() -> i32 + 'a>>>>,
  ) {
    let index = if let Some(get_operation_index) = get_operation_index {
      get_operation_index.borrow_mut()() as usize
    } else {
      context_block.operation.len()
    };
    context_block
      .operation
      .splice(index..index, vec![operation]);
  }

  pub fn push_template(self: &Self, content: String) -> i32 {
    let templates = &mut self.ir.borrow_mut().templates;
    if let Some(existing) = templates.iter().position(|i| i == &content) {
      return existing as i32;
    }
    let len = templates.len();
    templates.push(content);
    len as i32
  }

  pub fn register_template(self: &Self, dynamic: &mut IRDynamicInfo) -> i32 {
    let template = self.template.borrow();
    if template.is_empty() {
      return -1;
    }
    let id = self.push_template(template.clone());
    dynamic.template = Some(id);
    id
  }

  pub fn enter_block(
    self: &'a TransformContext<'a>,
    context_block: &'a mut BlockIRNode<'a>,
    ir: BlockIRNode<'a>,
    is_v_for: bool,
    exclude_slots: bool,
  ) -> Box<dyn FnOnce() -> BlockIRNode<'a> + 'a> {
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
      self.register_template(&mut context_block.dynamic);
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
      return_block
    }) as Box<dyn FnOnce() -> BlockIRNode<'a>>;

    exit_block
  }

  pub fn wrap_fragment(self: &Self, node: Expression<'a>) -> JSXChild<'a> {
    if let Expression::JSXFragment(node) = node {
      JSXChild::Fragment(node)
    } else if let Expression::JSXElement(node) = &node
      && is_template(node)
    {
      JSXChild::Element(node.clone_in(self.allocator))
    } else {
      JSXChild::Fragment(oxc_allocator::Box::new_in(
        JSXFragment {
          span: Span::new(0, 0),
          opening_fragment: JSXOpeningFragment {
            span: Span::new(0, 0),
          },
          closing_fragment: JSXClosingFragment {
            span: Span::new(0, 0),
          },
          children: oxc_allocator::Vec::from_array_in(
            [match node {
              Expression::JSXElement(node) => JSXChild::Element(node),
              Expression::JSXFragment(node) => JSXChild::Fragment(node),
              _ => JSXChild::ExpressionContainer(oxc_allocator::Box::new_in(
                JSXExpressionContainer {
                  span: Span::new(0, 0),
                  expression: to_jsx_expression(node),
                },
                self.allocator,
              )),
            }],
            self.allocator,
          ),
        },
        self.allocator,
      ))
    }
  }

  pub fn create_block(
    self: &'a Self,
    context_node: &mut ContextNode<'a>,
    context_block: &'a mut BlockIRNode<'a>,
    node: Expression<'a>,
    is_v_for: Option<bool>,
  ) -> Box<dyn FnOnce() -> BlockIRNode<'a> + 'a> {
    let block = BlockIRNode::new();
    *context_node = Either::B(self.wrap_fragment(node));
    let _context_block = context_block as *mut BlockIRNode;
    let exit_block = self.enter_block(
      unsafe { &mut *_context_block },
      block,
      is_v_for.unwrap_or(false),
      false,
    );
    self.reference(&mut context_block.dynamic);
    exit_block
  }

  pub fn create(
    self: &TransformContext<'a>,
    node: JSXChild<'a>,
    index: i32,
    block: &mut BlockIRNode<'a>,
  ) -> impl FnOnce() {
    self.node.replace(Either::B(node));
    let index = self.index.replace(index);
    let in_v_once = *self.in_v_once.borrow();
    let template = self.template.replace(String::new());
    self.children_template.take();
    mem::take(&mut block.dynamic);

    move || {
      self.index.replace(index);
      self.in_v_once.replace(in_v_once);
      self.template.replace(template);
      self.index.replace(index);
    }
  }

  pub fn transform_node<'b>(
    self: &'a TransformContext<'a>,
    context_block: Option<&'a mut BlockIRNode<'a>>,
  ) {
    let context_block = if let Some(context_block) = context_block {
      context_block
    } else {
      &mut self.block.borrow_mut()
    };

    let block = context_block as *mut BlockIRNode;
    let mut exit_fns = vec![];

    let is_root = matches!(&*self.node.borrow(), Either::A(_));
    if !is_root {
      for node_transform in vec![
        transform_v_once,
        transform_v_if,
        transform_v_for,
        transform_template_ref,
        transform_element,
        transform_text,
        transform_v_slots,
        transform_v_slot,
      ] {
        let on_exit = node_transform(&mut self.node.borrow_mut(), self, unsafe { &mut *block });
        if let Some(on_exit) = on_exit {
          exit_fns.push(on_exit);
        }
      }
    }

    transform_children(
      self.node.replace(Either::A(RootNode {
        is_fragment: false,
        children: oxc_allocator::Vec::new_in(self.allocator),
      })),
      self,
      unsafe { &mut *block },
    );

    let mut i = exit_fns.len();
    while i > 0 {
      i = i - 1;
      let on_exit = exit_fns.pop().unwrap();
      on_exit();
    }

    if is_root {
      self.register_template(&mut context_block.dynamic);
    }
  }
}

pub fn transform<'a>(
  allocator: &'a Allocator,
  node: RootNode<'a>,
  options: TransformOptions,
) -> VaporCodegenResult {
  let templates = options.templates.clone();
  let source = options.source.clone();
  let filename = options.filename.clone();
  let source_map = options.source_map;
  let ir = RootIRNode::new(source.clone(), templates.clone());

  let context = TransformContext::new(allocator, ir, Either::A(node), options);
  context.transform_node(None);

  let mut ir = context.ir.replace(RootIRNode::new(String::new(), vec![]));
  ir.block = context.block.take();
  generate(
    ir,
    CodegenOptions {
      filename: Some(filename),
      source_map: Some(source_map),
      templates: Some(templates),
    },
  )
}

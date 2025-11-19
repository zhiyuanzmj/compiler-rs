use std::collections::HashSet;

use napi::{
  Either,
  bindgen_prelude::{Either3, Either16},
};
use oxc_allocator::CloneIn;
use oxc_ast::ast::{ConditionalExpression, Expression, JSXChild, LogicalExpression};

use crate::{
  ir::index::{
    BlockIRNode, CreateNodesIRNode, DynamicFlag, GetTextChildIRNode, IfIRNode, SetNodesIRNode,
    SimpleExpressionNode,
  },
  transform::{ContextNode, TransformContext},
  utils::{
    check::{is_constant_node, is_fragment_node, is_jsx_component, is_template},
    text::{is_empty_text, resolve_jsx_text},
    utils::find_prop,
  },
};

pub fn transform_text<'a>(
  context_node: &mut ContextNode<'a>,
  context: &'a TransformContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
) -> Option<Box<dyn FnOnce() + 'a>> {
  let context_node = context_node as *mut ContextNode;
  let Either::B(node) = (unsafe { &*context_node }) else {
    return None;
  };
  let dynamic = &mut context_block.dynamic;
  let start = match node {
    JSXChild::Element(e) => e.span.start,
    JSXChild::ExpressionContainer(e) => e.span.start,
    JSXChild::Fragment(e) => e.span.start,
    JSXChild::Spread(e) => e.span.start,
    JSXChild::Text(e) => e.span.start,
  };
  let seen = &mut context.seen.borrow_mut();
  if seen.contains(&start) {
    dynamic.flags |= DynamicFlag::NonTemplate as i32;
    return None;
  }

  match node {
    JSXChild::Element(node) if !is_jsx_component(node) => process_children(
      &node.children.iter().collect::<Vec<_>>(),
      is_template(node),
      context,
      context_block,
      seen,
    ),
    JSXChild::Fragment(node) => process_children(
      &node.children.iter().collect::<Vec<_>>(),
      true,
      context,
      context_block,
      seen,
    ),
    JSXChild::ExpressionContainer(node) => {
      if let Some(expression) = node.expression.as_expression() {
        match expression.without_parentheses().get_inner_expression() {
          Expression::ConditionalExpression(expression) => {
            return Some(process_conditional_expression(
              expression,
              unsafe { &mut *context_node },
              context,
              context_block,
            ));
          }
          Expression::LogicalExpression(expression) => {
            return Some(process_logical_expression(
              expression,
              unsafe { &mut *context_node },
              context,
              context_block,
            ));
          }
          _ => process_interpolation(context, context_block, seen),
        }
      } else {
        dynamic.flags |= DynamicFlag::NonTemplate as i32;
      }
    }
    JSXChild::Text(node) => {
      let value = resolve_jsx_text(node);
      if !value.is_empty() {
        let mut template = context.template.borrow_mut();
        *template = template.to_string() + &value;
      } else {
        dynamic.flags |= DynamicFlag::NonTemplate as i32;
      }
    }
    _ => (),
  };
  None
}

fn process_children<'a>(
  children: &Vec<&JSXChild>,
  is_fragment: bool,
  context: &'a TransformContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
  seen: &mut HashSet<u32>,
) {
  if children.len() > 0 {
    let mut has_interp = false;
    let mut is_all_text_like = true;
    for child in children {
      if let JSXChild::ExpressionContainer(child) = child {
        let exp = child.expression.as_expression();
        if if let Some(exp) = exp {
          !matches!(
            exp.without_parentheses().get_inner_expression(),
            Expression::ConditionalExpression(_) | Expression::LogicalExpression(_),
          )
        } else {
          false
        } {
          has_interp = true
        }
      } else if !matches!(child, JSXChild::Text(_)) {
        is_all_text_like = false
      }
    }

    // all text like with interpolation
    if !is_fragment && is_all_text_like && has_interp {
      process_text_container(children, context, context_block, seen)
    } else if has_interp {
      // check if there's any text before interpolation, it needs to be merged
      let mut i = 0;
      for child in children {
        let prev = if i > 0 { children.get(i - 1) } else { None };
        if let JSXChild::ExpressionContainer(_) = child
          && let Some(JSXChild::Text(_)) = prev
        {
          // mark leading text node for skipping
          mark_non_template(prev.unwrap(), seen);
        }
        i = i + 1;
      }
    }
  }
}

fn process_interpolation<'a>(
  context: &'a TransformContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
  seen: &mut HashSet<u32>,
) {
  let Some(parent_node) = &*context.parent_node.borrow() else {
    return;
  };
  let children = match parent_node {
    ContextNode::A(e) => &e.children,
    ContextNode::B(e) => match e {
      JSXChild::Element(e) => &e.children,
      JSXChild::Fragment(e) => &e.children,
      _ => return,
    },
  };
  if children.len() == 0 {
    return;
  }
  let index = *context.index.borrow() as usize;
  let nexts = children[index..].iter().collect::<Vec<_>>();
  let idx = nexts.iter().position(|n| !is_text_like(n));
  let mut nodes = if let Some(idx) = idx {
    nexts[..idx].to_vec()
  } else {
    nexts
  };

  // merge leading text
  let prev = if index > 0 {
    children.get(index - 1)
  } else {
    None
  };
  if let Some(prev) = prev
    && let JSXChild::Text(_) = prev
  {
    nodes.insert(0, prev);
  }

  let values = process_text_like_expressions(&nodes, context, seen);
  let dynamic = &mut context_block.dynamic;
  if values.is_empty() {
    dynamic.flags |= DynamicFlag::NonTemplate as i32;
    return;
  }

  let id = context.reference(dynamic);
  let once = *context.in_v_once.borrow();
  if match parent_node {
    Either::A(_) => true,
    Either::B(parent) => {
      is_fragment_node(parent)
        || matches!(parent, JSXChild::Element(parent) if find_prop(parent, Either::A(String::from("v-slot"))).is_some())
    }
  } {
    context.register_operation(
      context_block,
      Either16::K(CreateNodesIRNode {
        create_nodes: true,
        id,
        once,
        values,
      }),
      None,
    );
  } else {
    let mut template = context.template.borrow_mut();
    *template = template.to_string() + " ";
    context.register_operation(
      context_block,
      Either16::G(SetNodesIRNode {
        set_nodes: true,
        element: id,
        once,
        values,
        generated: None,
      }),
      None,
    );
  };
}

fn mark_non_template<'a>(node: &JSXChild, seen: &'a mut HashSet<u32>) {
  // let seen = &mut context.seen.borrow_mut();
  seen.insert(match node {
    JSXChild::Element(e) => e.span.start,
    JSXChild::Fragment(e) => e.span.start,
    JSXChild::ExpressionContainer(e) => e.span.start,
    JSXChild::Spread(e) => e.span.start,
    JSXChild::Text(e) => e.span.start,
  });
}

fn process_text_container<'a>(
  children: &Vec<&JSXChild>,
  context: &'a TransformContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
  seen: &mut HashSet<u32>,
) {
  let values = process_text_like_expressions(children, context, seen);
  let literals = values
    .iter()
    .map(|e| e.get_literal_expression_value())
    .collect::<Vec<Option<String>>>();
  if literals.iter().all(|l| l.is_some()) {
    *context.children_template.borrow_mut() = literals.into_iter().filter_map(|i| i).collect();
  } else {
    *context.children_template.borrow_mut() = vec![" ".to_string()];
    let parent = context.reference(&mut context_block.dynamic);
    context.register_operation(
      context_block,
      Either16::P(GetTextChildIRNode {
        get_text_child: true,
        parent,
      }),
      None,
    );
    let element = context.reference(&mut context_block.dynamic);
    context.register_operation(
      context_block,
      Either16::G(SetNodesIRNode {
        set_nodes: true,
        element,
        once: *context.in_v_once.borrow(),
        values,
        // indicates this node is generated, so prefix should be "x" instead of "n"
        generated: Some(true),
      }),
      None,
    );
  }
}

fn process_text_like_expressions<'a>(
  nodes: &Vec<&JSXChild>,
  context: &'a TransformContext<'a>,
  seen: &mut HashSet<u32>,
) -> Vec<SimpleExpressionNode<'a>> {
  let mut values = vec![];
  for node in nodes {
    mark_non_template(node, seen);
    if is_empty_text(node) {
      continue;
    }
    values.push(SimpleExpressionNode::new(Either3::B(node), context))
  }
  values
}

fn is_text_like(node: &JSXChild) -> bool {
  if let JSXChild::ExpressionContainer(node) = node {
    !matches!(
      node
        .expression
        .to_expression()
        .without_parentheses()
        .get_inner_expression(),
      Expression::ConditionalExpression(_) | Expression::LogicalExpression(_)
    )
  } else {
    matches!(node, JSXChild::Text(_))
  }
}

pub fn process_conditional_expression<'a>(
  node: &'a ConditionalExpression,
  context_node: &'a mut ContextNode<'a>,
  context: &'a TransformContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
) -> Box<dyn FnOnce() + 'a> {
  let test = &node.test;
  let consequent = node
    .consequent
    .without_parentheses()
    .get_inner_expression()
    .clone_in(context.allocator);
  let alternate = node.alternate.without_parentheses().get_inner_expression();

  let dynamic = &mut context_block.dynamic;
  dynamic.flags = dynamic.flags | DynamicFlag::NonTemplate as i32 | DynamicFlag::Insert as i32;
  let id = context.reference(dynamic);
  let block = context_block as *mut BlockIRNode;
  let exit_block = context.create_block(context_node, unsafe { &mut *block }, consequent, None);

  let is_const_test = is_constant_node(&Some(&test));
  let test = SimpleExpressionNode::new(Either3::A(&test), context);

  Box::new(move || {
    let block = exit_block();

    let mut operation = IfIRNode {
      id,
      positive: block,
      once: *context.in_v_once.borrow() || is_const_test,
      condition: test,
      negative: None,
      parent: None,
      anchor: None,
    };
    let _context_block = context_block as *mut BlockIRNode;
    set_negative(alternate, &mut operation, context_node, context, unsafe {
      &mut *_context_block
    });
    context_block.dynamic.operation = Some(Box::new(Either16::A(operation)));
  })
}

fn process_logical_expression<'a>(
  node: &'a LogicalExpression,
  context_node: &'a mut ContextNode<'a>,
  context: &'a TransformContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
) -> Box<dyn FnOnce() + 'a> {
  let left = node.left.without_parentheses().get_inner_expression();
  let right = node.right.without_parentheses().get_inner_expression();
  let operator_is_and = node.operator.is_and();

  let dynamic = &mut context_block.dynamic;
  dynamic.flags = dynamic.flags | DynamicFlag::NonTemplate as i32 | DynamicFlag::Insert as i32;
  let id = context.reference(dynamic);
  let block = context_block as *mut BlockIRNode;
  let exit_block = context.create_block(
    context_node,
    unsafe { &mut *block },
    if operator_is_and {
      right.clone_in(context.allocator)
    } else {
      left.clone_in(context.allocator)
    },
    None,
  );

  Box::new(move || {
    let block = exit_block();

    let mut operation = IfIRNode {
      id,
      positive: block,
      once: *context.in_v_once.borrow() || is_constant_node(&Some(&left)),
      condition: SimpleExpressionNode::new(Either3::A(&left), context),
      negative: None,
      anchor: None,
      parent: None,
    };
    let _context_block = context_block as *mut BlockIRNode;

    set_negative(
      if operator_is_and { left } else { right },
      &mut operation,
      context_node,
      context,
      unsafe { &mut *_context_block },
    );

    context_block.dynamic.operation = Some(Box::new(Either16::A(operation)));
  })
}

fn set_negative<'a>(
  node: &Expression<'a>,
  operation: &mut IfIRNode<'a>,
  context_node: &mut ContextNode<'a>,
  context: &'a TransformContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
) {
  let node = node.without_parentheses().get_inner_expression();
  if let Expression::ConditionalExpression(node) = node {
    let _context_block = context_block as *mut BlockIRNode;
    let exit_block = context.create_block(
      context_node,
      unsafe { &mut *_context_block },
      node
        .consequent
        .without_parentheses()
        .get_inner_expression()
        .clone_in(context.allocator),
      None,
    );
    context.transform_node(Some(unsafe { &mut *_context_block }));
    let block = exit_block();
    let mut negative = IfIRNode {
      id: -1,
      condition: SimpleExpressionNode::new(Either3::A(&node.test), context),
      positive: block,
      once: *context.in_v_once.borrow() || is_constant_node(&Some(&node.test)),
      negative: None,
      anchor: None,
      parent: None,
    };
    set_negative(
      &node.alternate,
      &mut negative,
      context_node,
      context,
      context_block,
    );
    operation.negative = Some(Box::new(Either::B(negative)));
  } else if let Expression::LogicalExpression(node) = node {
    let left = node.left.without_parentheses().get_inner_expression();
    let right = node.right.without_parentheses().get_inner_expression();
    let operator_is_and = node.operator.is_and();
    let block = context_block as *mut BlockIRNode;
    let exit_block = context.create_block(
      context_node,
      unsafe { &mut *block },
      if operator_is_and {
        right.clone_in(context.allocator)
      } else {
        left.clone_in(context.allocator)
      },
      None,
    );
    context.transform_node(Some(unsafe { &mut *block }));
    let block = exit_block();
    let mut negative = IfIRNode {
      id: -1,
      condition: SimpleExpressionNode::new(Either3::A(&left), context),
      positive: block,
      once: *context.in_v_once.borrow() || is_constant_node(&Some(&left)),
      negative: None,
      anchor: None,
      parent: None,
    };
    set_negative(
      if operator_is_and { left } else { right },
      &mut negative,
      context_node,
      context,
      context_block,
    );
    operation.negative = Some(Box::new(Either::B(negative)));
  } else {
    let block = context_block as *mut BlockIRNode;
    let exit_block = context.create_block(
      context_node,
      unsafe { &mut *block },
      node.clone_in(context.allocator),
      None,
    );
    context.transform_node(Some(context_block));
    let block = exit_block();
    operation.negative = Some(Box::new(Either::A(block)));
  }
}

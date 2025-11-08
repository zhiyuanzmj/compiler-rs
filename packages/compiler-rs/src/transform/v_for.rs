use std::rc::Rc;

use napi::{
  Either,
  bindgen_prelude::{Either3, Either16},
};
use oxc_allocator::CloneIn;
use oxc_ast::ast::{Expression, JSXAttribute, JSXAttributeValue, JSXChild, JSXElement};

use crate::{
  ir::index::{BlockIRNode, DynamicFlag, ForIRNode, IRFor, SimpleExpressionNode},
  transform::TransformContext,
  utils::{
    check::{is_constant_node, is_jsx_component, is_template},
    error::{ErrorCodes, on_error},
    text::is_empty_text,
    utils::find_prop,
  },
};

pub fn transform_v_for<'a>(
  node: &JSXChild,
  context: &'a Rc<TransformContext<'a>>,
  context_block: &'a mut BlockIRNode<'a>,
) -> Option<Box<dyn FnOnce() + 'a>> {
  let JSXChild::Element(node) = node else {
    return None;
  };
  if is_template(node) && find_prop(node, Either::A("v-slot".to_string())).is_some() {
    return None;
  }

  let Some(dir) = find_prop(&node, Either::A("v-for".to_string())) else {
    return None;
  };
  let seen = &mut context.seen.borrow_mut();
  let start = dir.span.start;
  if seen.contains(&start) {
    return None;
  }
  seen.insert(start);

  let IRFor {
    value,
    index,
    key,
    source,
  } = get_for_parse_result(dir, context);

  let Some(source) = source else {
    on_error(ErrorCodes::VForMalformedExpression, context);
    return None;
  };

  let key_prop = find_prop(&node, Either::A("key".to_string()));
  let key_prop = if let Some(key_prop) = key_prop
    && let Some(value) = &key_prop.value
  {
    Some(SimpleExpressionNode::new(Either3::C(value), context))
  } else {
    None
  };

  let component = is_jsx_component(node) || is_template_with_single_component(node);
  let dynamic = &mut context_block.dynamic;
  let id = context.reference(dynamic);
  dynamic.flags = dynamic.flags | DynamicFlag::NonTemplate as i32 | DynamicFlag::Insert as i32;
  let block = context_block as *mut BlockIRNode;
  let exit_block = context.create_block(
    unsafe { &mut *block },
    Expression::JSXElement(node.clone_in(context.allocator)),
    Some(true),
  );

  Some(Box::new(move || {
    let block = exit_block();

    // if v-for is the only child of a parent element, it can go the fast path
    // when the entire list is emptied
    let only_child = if let Some(Either::B(JSXChild::Element(parent_node))) =
      &*context.parent_node.borrow()
      && !is_jsx_component(parent_node)
      && parent_node
        .children
        .iter()
        .filter(|child| !is_empty_text(child))
        .collect::<Vec<_>>()
        .len()
        == 1
    {
      true
    } else {
      false
    };

    context_block.dynamic.operation = Some(Box::new(Either16::B(ForIRNode {
      id,
      value,
      key,
      index,
      key_prop,
      render: block,
      once: *context.in_v_once.borrow() || is_constant_node(&source.ast.as_ref()),
      source,
      component,
      only_child,
      parent: None,
      anchor: None,
    })));
  }))
}

pub fn get_for_parse_result<'a>(
  dir: &JSXAttribute,
  context: &'a Rc<TransformContext<'a>>,
) -> IRFor<'a> {
  let mut value: Option<SimpleExpressionNode> = None;
  let mut index: Option<SimpleExpressionNode> = None;
  let mut key: Option<SimpleExpressionNode> = None;
  let mut source: Option<SimpleExpressionNode> = None;
  if let Some(dir_value) = &dir.value {
    let expression = if let JSXAttributeValue::ExpressionContainer(dir_value) = dir_value {
      Some(
        dir_value
          .expression
          .to_expression()
          .without_parentheses()
          .get_inner_expression(),
      )
    } else {
      None
    };
    if let Some(expression) = expression
      && let Expression::BinaryExpression(expression) = expression
    {
      let left = expression.left.without_parentheses().get_inner_expression();
      if let Expression::SequenceExpression(left) = left {
        let expressions = &left.expressions;
        value = expressions
          .get(0)
          .map(|e| SimpleExpressionNode::new(Either3::A(e), context));
        key = expressions
          .get(1)
          .map(|e| SimpleExpressionNode::new(Either3::A(e), context));
        index = expressions
          .get(2)
          .map(|e| SimpleExpressionNode::new(Either3::A(e), context));
      } else {
        value = Some(SimpleExpressionNode::new(Either3::A(left), context));
      };
      source = Some(SimpleExpressionNode::new(
        Either3::A(&expression.right),
        context,
      ));
    }
  } else {
    on_error(ErrorCodes::VForNoExpression, context);
  }
  return IRFor {
    value,
    index,
    key,
    source,
  };
}

fn is_template_with_single_component<'a>(node: &'a JSXElement<'a>) -> bool {
  let non_comment_children = node
    .children
    .iter()
    .filter(|c| !is_empty_text(c))
    .collect::<Vec<_>>();

  non_comment_children.len() == 1
    && matches!(non_comment_children[0],JSXChild::Element(child)if is_jsx_component(child))
}

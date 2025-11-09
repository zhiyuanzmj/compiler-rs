use napi::{Either, bindgen_prelude::Either16};
use oxc_allocator::CloneIn;
use oxc_ast::ast::{Expression, JSXChild};

use crate::{
  ir::index::{BlockIRNode, DynamicFlag, IRDynamicInfo, IfIRNode, SimpleExpressionNode},
  transform::TransformContext,
  utils::{
    check::{is_constant_node, is_template},
    directive::resolve_directive,
    error::{ErrorCodes, on_error},
    utils::find_prop,
  },
};

pub fn transform_v_if<'a>(
  node: &JSXChild,
  context: &'a TransformContext<'a>,
  context_block: &'a mut BlockIRNode<'a>,
) -> Option<Box<dyn FnOnce() + 'a>> {
  let JSXChild::Element(node) = node else {
    return None;
  };
  if is_template(node) && find_prop(node, Either::A("v-slot".to_string())).is_some() {
    return None;
  }

  let Some(dir) = find_prop(
    &node,
    Either::B(vec![
      "v-if".to_string(),
      "v-else".to_string(),
      "v-else-if".to_string(),
    ]),
  ) else {
    return None;
  };
  let seen = &mut context.seen.borrow_mut();
  let start = dir.span.start;
  if seen.contains(&start) {
    return None;
  }
  seen.insert(start);

  let mut dir = resolve_directive(dir, context);
  if dir.name != "else"
    && (dir.exp.is_none() || dir.exp.as_ref().unwrap().content.trim().is_empty())
  {
    on_error(ErrorCodes::VIfNoExpression, context);
    dir.exp = Some(SimpleExpressionNode {
      content: "true".to_string(),
      is_static: false,
      loc: None,
      ast: None,
    });
  }

  let dynamic = &mut context_block.dynamic;
  dynamic.flags |= DynamicFlag::NonTemplate as i32;

  if dir.name == "if" {
    let id = context.reference(dynamic);
    dynamic.flags |= DynamicFlag::Insert as i32;
    let block = context_block as *mut BlockIRNode;
    let exit_block = context.create_block(
      unsafe { &mut *block },
      Expression::JSXElement(node.clone_in(context.allocator)),
      None,
    );
    return Some(Box::new(move || {
      let block = exit_block();

      context_block.dynamic.operation = Some(Box::new(Either16::A(IfIRNode {
        id,
        positive: block,
        once: Some(
          *context.in_v_once.borrow() || is_constant_node(&dir.exp.as_ref().unwrap().ast.as_ref()),
        ),
        condition: dir.exp.unwrap(),
        negative: None,
        anchor: None,
        parent: None,
      })));
    }));
  }

  let siblings = &mut context.parent_dynamic.borrow_mut().children;
  let mut last_if_node = None;
  if siblings.len() > 0 {
    let mut i = siblings.len();
    while i > 0 {
      i = i - 1;
      let sibling = siblings.get_mut(i).unwrap() as *mut IRDynamicInfo;
      if let Some(operation) = (unsafe { &mut *sibling }).operation.as_mut()
        && let Either16::A(operation) = operation.as_mut()
      {
        last_if_node = Some(operation);
        break;
      }
    }
  }

  // check if IfNode is the last operation and get the root IfNode
  let Some(mut last_if_node) = last_if_node else {
    on_error(ErrorCodes::VElseNoAdjacentIf, context);
    return None;
  };

  let mut last_if_node_ptr = last_if_node as *mut IfIRNode;
  while let Some(negative) = (unsafe { &mut *last_if_node_ptr }).negative.as_mut()
    && let Either::B(negative) = negative.as_mut()
  {
    last_if_node_ptr = negative as *mut IfIRNode;
  }
  last_if_node = unsafe { &mut *last_if_node_ptr };

  // Check if v-else was followed by v-else-if
  if dir.name == "else-if" && last_if_node.negative.is_some() {
    on_error(ErrorCodes::VElseNoAdjacentIf, context);
  };

  let exit_block = context.create_block(
    context_block,
    Expression::JSXElement(node.clone_in(context.allocator)),
    None,
  );

  Some(Box::new(move || {
    let block = exit_block();
    if dir.name == "else" {
      last_if_node.negative = Some(Box::new(Either::A(block)));
    } else {
      last_if_node.negative = Some(Box::new(Either::B(IfIRNode {
        id: -1,
        positive: block,
        once: Some(
          *context.in_v_once.borrow() || is_constant_node(&dir.exp.as_ref().unwrap().ast.as_ref()),
        ),
        condition: dir.exp.unwrap(),
        parent: None,
        anchor: None,
        negative: None,
      })))
    }
  }))
}

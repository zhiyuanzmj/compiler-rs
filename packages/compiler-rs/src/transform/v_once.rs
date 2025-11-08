use std::rc::Rc;

use napi::Either;
use oxc_ast::ast::JSXChild;

use crate::{ir::index::BlockIRNode, transform::TransformContext, utils::utils::find_prop};

pub fn transform_v_once<'a>(
  node: &JSXChild,
  context: &'a Rc<TransformContext<'a>>,
  _: &'a mut BlockIRNode<'a>,
) -> Option<Box<dyn FnOnce() + 'a>> {
  if let JSXChild::Element(node) = &node
    && find_prop(node, Either::A(String::from("v-once"))).is_some()
  {
    *context.in_v_once.borrow_mut() = true;
  }
  None
}

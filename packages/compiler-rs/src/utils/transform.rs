use napi::bindgen_prelude::Object;
use napi_derive::napi;

use crate::ir::index::{BlockIRNode, DynamicFlag, IRDynamicInfo, IRNodeTypes};

#[napi]
pub fn new_dynamic() -> IRDynamicInfo {
  return IRDynamicInfo {
    flags: DynamicFlag::REFERENCED,
    children: Vec::new(),
    id: None,
    anchor: None,
    template: None,
    has_dynamic_child: None,
    operation: None,
  };
}

#[napi]
pub fn new_block(node: Object<'static>) -> BlockIRNode {
  BlockIRNode {
    _type: IRNodeTypes::BLOCK,
    node,
    dynamic: new_dynamic(),
    effect: Vec::new(),
    operation: Vec::new(),
    returns: Vec::new(),
    temp_id: 0,
  }
}

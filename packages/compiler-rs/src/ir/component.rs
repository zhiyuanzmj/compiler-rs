use std::collections::HashMap;

use napi::{
  Either,
  bindgen_prelude::{Either3, Object},
};
use napi_derive::napi;

use crate::{
  ir::index::{
    DynamicFlag, IRDynamicInfo, IREffect, IRFor, IRNodeTypes, Modifiers, OperationNode,
    SimpleExpressionNode,
  },
  utils::my_box::MyBox,
};

#[napi(object, js_name = "IRProp")]
pub struct IRProp {
  pub key: SimpleExpressionNode,
  #[napi(ts_type = "'.' | '^'")]
  pub modifier: Option<String>,
  pub runtime_camelize: Option<bool>,
  pub handler: Option<bool>,
  pub handler_modifiers: Option<Modifiers>,
  pub model: Option<bool>,
  pub model_modifiers: Option<Vec<String>>,

  pub values: Vec<SimpleExpressionNode>,
}

#[napi(js_name = "IRDynamicPropsKind")]
pub enum IRDynamicPropsKind {
  EXPRESSION, // v-bind="value"
  ATTRIBUTE,  // v-bind:[foo]="value"
}

#[napi]
pub type IRPropsStatic = Vec<IRProp>;

#[napi(object, js_name = "IRPropsDynamicExpression")]
pub struct IRPropsDynamicExpression {
  #[napi(ts_type = "IRDynamicPropsKind.EXPRESSION")]
  pub kind: IRDynamicPropsKind,
  pub value: SimpleExpressionNode,
  pub handler: Option<bool>,
}

#[napi(object, js_name = "IRPropsDynamicAttribute")]
pub struct IRPropsDynamicAttribute {
  pub key: SimpleExpressionNode,
  #[napi(ts_type = "'.' | '^'")]
  pub modifier: Option<String>,
  pub runtime_camelize: Option<bool>,
  pub handler: Option<bool>,
  pub handler_modifiers: Option<Modifiers>,
  pub model: Option<bool>,
  pub model_modifiers: Option<Vec<String>>,
  pub values: Vec<SimpleExpressionNode>,

  #[napi(ts_type = "IRDynamicPropsKind.ATTRIBUTE")]
  pub kind: IRDynamicPropsKind,
}

#[napi]
pub type IRProps = Either3<IRPropsStatic, IRPropsDynamicAttribute, IRPropsDynamicExpression>;

// slots
#[napi(object, js_name = "SlotBlockIRNode")]
pub struct SlotBlockIRNode {
  #[napi(ts_type = "IRNodeTypes.BLOCK")]
  pub _type: IRNodeTypes,
  #[napi(ts_type = "import('oxc-parser').Node")]
  pub node: Object<'static>,
  pub dynamic: IRDynamicInfo,
  pub temp_id: i32,
  pub effect: Vec<IREffect>,
  pub operation: Vec<OperationNode>,
  pub returns: Vec<i32>,
  pub props: Option<SimpleExpressionNode>,
}
impl SlotBlockIRNode {
  pub fn new(node: Object<'static>, props: Option<SimpleExpressionNode>) -> Self {
    SlotBlockIRNode {
      _type: IRNodeTypes::BLOCK,
      node,
      dynamic: IRDynamicInfo::new(),
      effect: Vec::new(),
      operation: Vec::new(),
      returns: Vec::new(),
      temp_id: 0,
      props,
    }
  }
}

#[napi]
pub enum IRSlotType {
  STATIC,
  DYNAMIC,
  LOOP,
  CONDITIONAL,
  EXPRESSION,
}

#[napi(object, js_name = "IRSlotsStatic")]
pub struct IRSlotsStatic {
  #[napi(ts_type = "IRSlotType.STATIC")]
  pub slot_type: IRSlotType,
  pub slots: HashMap<String, SlotBlockIRNode>,
}

#[napi(object, js_name = "IRSlotDynamicBasic")]
pub struct IRSlotDynamicBasic {
  #[napi(ts_type = "IRSlotType.DYNAMIC")]
  pub slot_type: IRSlotType,
  pub name: SimpleExpressionNode,
  #[napi(ts_type = "SlotBlockIRNode")]
  pub _fn: Object<'static>,
  // should removed
  pub _loop: Option<IRFor>,
}

#[napi(object, js_name = "IRSlotDynamicLoop")]
pub struct IRSlotDynamicLoop {
  #[napi(ts_type = "IRSlotType.LOOP")]
  pub slot_type: IRSlotType,
  pub name: SimpleExpressionNode,
  #[napi(ts_type = "SlotBlockIRNode")]
  pub _fn: Object<'static>,
  pub _loop: IRFor,
}

#[napi(object, js_name = "IRSlotDynamicConditional")]
pub struct IRSlotDynamicConditional {
  #[napi(ts_type = "IRSlotType.CONDITIONAL")]
  pub slot_type: IRSlotType,
  pub condition: SimpleExpressionNode,
  pub positive: IRSlotDynamicBasic,
  #[napi(ts_type = "IRSlotDynamicBasic | IRSlotDynamicConditional")]
  pub negative: Option<MyBox<Either<IRSlotDynamicBasic, IRSlotDynamicConditional>>>,
}

#[napi(object, js_name = "IRSlotsExpression")]
pub struct IRSlotsExpression {
  #[napi(ts_type = "IRSlotType.EXPRESSION")]
  pub slot_type: IRSlotType,
  pub slots: SimpleExpressionNode,
}

#[napi]
pub type IRSlotDynamic = Either3<IRSlotDynamicBasic, IRSlotDynamicLoop, IRSlotDynamicConditional>;

#[napi(js_name = "IRSlots")]
pub type IRSlots = Either3<IRSlotsStatic, IRSlotDynamic, IRSlotsExpression>;

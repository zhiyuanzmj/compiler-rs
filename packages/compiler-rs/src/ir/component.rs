use std::collections::HashMap;

use napi::{
  Either,
  bindgen_prelude::{Either3, Either4},
};
use napi_derive::napi;

use crate::{
  ir::index::{BlockIRNode, IRFor, Modifiers, SimpleExpressionNode},
  utils::my_box::MyBox,
};

#[napi(object, js_name = "IRProp")]
#[derive(Clone)]
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
  pub dynamic: bool,
}

#[napi]
pub type IRPropsStatic = Vec<IRProp>;

#[napi(object, js_name = "IRPropsDynamicExpression")]
#[derive(Clone)]
pub struct IRPropsDynamicExpression {
  pub value: SimpleExpressionNode,
  pub handler: Option<bool>,
}

#[napi]
pub type IRProps = Either3<IRPropsStatic, IRProp, IRPropsDynamicExpression>;

// slots
#[napi]
pub enum IRSlotType {
  STATIC,
  DYNAMIC,
  CONDITIONAL,
  EXPRESSION,
}

#[napi(object, js_name = "IRSlotsStatic")]
pub struct IRSlotsStatic {
  #[napi(ts_type = "IRSlotType.STATIC")]
  pub slot_type: IRSlotType,
  pub slots: HashMap<String, BlockIRNode>,
}

#[napi(object, js_name = "IRSlotDynamicBasic")]
pub struct IRSlotDynamicBasic {
  #[napi(ts_type = "IRSlotType.DYNAMIC")]
  pub slot_type: IRSlotType,
  pub name: SimpleExpressionNode,
  pub _fn: BlockIRNode,
  pub _loop: Option<IRFor>,
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

// #[napi]
// pub type IRSlotDynamic = Either<IRSlotDynamicBasic, IRSlotDynamicConditional>;

#[napi(js_name = "IRSlots")]
pub type IRSlots =
  Either4<IRSlotsStatic, IRSlotDynamicBasic, IRSlotDynamicConditional, IRSlotsExpression>;

use std::collections::HashSet;

use napi::{
  Either,
  bindgen_prelude::{Either3, Either18, Object},
};
use napi_derive::napi;

use crate::{
  ir::component::{IRProp, IRProps, IRSlots},
  utils::my_box::MyBox,
};

#[napi]
pub enum IRNodeTypes {
  ROOT = 0,
  BLOCK,

  SET_PROP,
  SET_DYNAMIC_PROPS,
  SET_TEXT,
  SET_EVENT,
  SET_DYNAMIC_EVENTS,
  SET_HTML,
  SET_TEMPLATE_REF,

  INSERT_NODE,
  PREPEND_NODE,
  CREATE_COMPONENT_NODE,
  SLOT_OUTLET_NODE,

  DIRECTIVE,
  DECLARE_OLD_REF, // consider make it more general

  IF,
  FOR,

  GET_TEXT_CHILD,

  CREATE_NODES,
  SET_NODES,
}

#[napi(object, js_name = "BaseIRNode")]
pub struct BaseIRNode {
  pub _type: IRNodeTypes,
}

#[napi(object)]
pub struct RootNode {
  #[napi(ts_type = "IRNodeTypes.ROOT | 'JSXFragment'")]
  pub _type: Either<IRNodeTypes, String>,
  pub source: String,
  #[napi(ts_type = "Array<import('oxc-parser').JSXChild>")]
  pub children: Vec<Object<'static>>,
}

#[napi(object, js_name = "BlockIRNode")]
pub struct BlockIRNode {
  #[napi(ts_type = "IRNodeTypes.BLOCK")]
  pub _type: IRNodeTypes,
  #[napi(ts_type = "RootNode | import('oxc-parser').Node")]
  pub node: Object<'static>,
  pub dynamic: IRDynamicInfo,
  pub temp_id: i32,
  pub effect: Vec<IREffect>,
  pub operation: Vec<OperationNode>,
  pub returns: Vec<i32>,
}

#[napi(object, js_name = "RootIRNode")]
pub struct RootIRNode {
  #[napi(ts_type = "IRNodeTypes.ROOT")]
  pub _type: IRNodeTypes,
  pub node: RootNode,
  pub source: String,
  pub templates: Vec<String>,
  pub root_template_index: Option<i32>,
  pub component: HashSet<String>,
  pub directive: HashSet<String>,
  pub block: BlockIRNode,
  pub has_template_ref: bool,
}

#[napi(object, js_name = "IfIRNode")]
pub struct IfIRNode {
  #[napi(ts_type = "IRNodeTypes.IF")]
  pub _type: IRNodeTypes,
  pub id: i32,
  pub condition: SimpleExpressionNode,
  pub positive: BlockIRNode,
  #[napi(ts_type = "BlockIRNode | IfIRNode")]
  pub negative: Option<MyBox<Either<BlockIRNode, IfIRNode>>>,
  pub once: Option<bool>,
  pub parent: Option<i32>,
  pub anchor: Option<i32>,
}

#[napi(object, js_name = "IRFor")]
pub struct IRFor {
  pub source: SimpleExpressionNode,
  pub value: Option<SimpleExpressionNode>,
  pub key: Option<SimpleExpressionNode>,
  pub index: Option<SimpleExpressionNode>,
}

#[napi(object, js_name = "ForIRNode")]
pub struct ForIRNode {
  pub source: SimpleExpressionNode,
  pub value: Option<SimpleExpressionNode>,
  pub key: Option<SimpleExpressionNode>,
  pub index: Option<SimpleExpressionNode>,

  #[napi(ts_type = "IRNodeTypes.FOR")]
  pub _type: IRNodeTypes,
  pub id: i32,
  pub key_prop: Option<SimpleExpressionNode>,
  pub render: BlockIRNode,
  pub once: bool,
  pub component: bool,
  pub only_child: bool,
  pub parent: Option<i32>,
  pub anchor: Option<i32>,
}

#[napi(object, js_name = "SetPropIRNode")]
pub struct SetPropIRNode {
  #[napi(ts_type = "IRNodeTypes.SET_PROP")]
  pub _type: IRNodeTypes,
  pub element: i32,
  pub prop: IRProp,
  pub root: bool,
  pub tag: String,
}

#[napi(object, js_name = "SetDynamicPropsIRNode")]
pub struct SetDynamicPropsIRNode {
  #[napi(ts_type = "IRNodeTypes.SET_DYNAMIC_PROPS")]
  pub _type: IRNodeTypes,
  pub element: i32,
  pub props: Vec<IRProps>,
  pub root: bool,
}

#[napi(object, js_name = "SetDynamicEventsIRNode")]
pub struct SetDynamicEventsIRNode {
  #[napi(ts_type = "IRNodeTypes.SET_DYNAMIC_EVENTS")]
  pub _type: IRNodeTypes,
  pub element: i32,
  pub event: SimpleExpressionNode,
}

#[napi(object, js_name = "SetTextIRNode")]
pub struct SetTextIRNode {
  #[napi(ts_type = "IRNodeTypes.SET_TEXT")]
  pub _type: IRNodeTypes,
  pub element: i32,
  pub values: Vec<SimpleExpressionNode>,
  pub generated: Option<bool>,
}

#[napi(object, js_name = "SetNodesIRNode")]
pub struct SetNodesIRNode {
  #[napi(ts_type = "IRNodeTypes.SET_NODES")]
  pub _type: IRNodeTypes,
  pub element: i32,
  pub once: bool,
  pub values: Vec<SimpleExpressionNode>,
  pub generated: Option<bool>, // whether this is a generated empty text node by `processTextLikeContainer`
}

#[napi(array)]
pub struct KeyOverride(
  #[napi(js_name = "find")] pub String,
  #[napi(js_name = "replacement")] pub String,
);

#[napi(object)]
pub struct Modifiers {
  // modifiers for addEventListener() options, e.g. .passive & .capture
  pub options: Vec<String>,
  // modifiers that needs runtime guards, withKeys
  pub keys: Vec<String>,
  // modifiers that needs runtime guards, withModifiers
  pub non_keys: Vec<String>,
}

#[napi(object, js_name = "SetEventIRNode")]
pub struct SetEventIRNode {
  #[napi(ts_type = "IRNodeTypes.SET_EVENT")]
  pub _type: IRNodeTypes,
  pub element: i32,
  pub key: SimpleExpressionNode,
  pub value: Option<SimpleExpressionNode>,
  pub modifiers: Modifiers,
  pub key_override: Option<KeyOverride>,
  pub delegate: bool,
  // Whether it's in effect
  pub effect: bool,
}

#[napi(object, js_name = "SetHtmlIRNode")]
pub struct SetHtmlIRNode {
  #[napi(ts_type = "IRNodeTypes.SET_HTML")]
  pub _type: IRNodeTypes,
  pub element: i32,
  pub value: SimpleExpressionNode,
}

#[napi(object, js_name = "SetTemplateRefIRNode")]
pub struct SetTemplateRefIRNode {
  #[napi(ts_type = "IRNodeTypes.SET_TEMPLATE_REF")]
  pub _type: IRNodeTypes,
  pub element: i32,
  pub value: SimpleExpressionNode,
  pub ref_for: bool,
  pub effect: bool,
}

#[napi(object, js_name = "CreateNodesIRNode")]
pub struct CreateNodesIRNode {
  #[napi(ts_type = "IRNodeTypes.CREATE_NODES")]
  pub _type: IRNodeTypes,
  pub id: i32,
  pub once: bool,
  pub values: Option<Vec<SimpleExpressionNode>>,
}

#[napi(object, js_name = "InsertNodeIRNode")]
pub struct InsertNodeIRNode {
  #[napi(ts_type = "IRNodeTypes.INSERT_NODE")]
  pub _type: IRNodeTypes,
  pub elements: Vec<i32>,
  pub parent: i32,
  pub anchor: Option<i32>,
}

#[napi(object, js_name = "PrependNodeIRNode")]
pub struct PrependNodeIRNode {
  #[napi(ts_type = "IRNodeTypes.PREPEND_NODE")]
  pub _type: IRNodeTypes,
  pub elements: Vec<i32>,
  pub parent: i32,
}

#[napi(object, js_name = "DirectiveIRNode")]
pub struct DirectiveIRNode {
  #[napi(ts_type = "IRNodeTypes.DIRECTIVE")]
  pub _type: IRNodeTypes,
  pub element: i32,
  pub dir: DirectiveNode,
  pub name: String,
  pub builtin: Option<bool>,
  pub asset: Option<bool>,
  #[napi(ts_type = "'text' | 'dynamic' | 'radio' | 'checkbox' | 'select'")]
  pub model_type: Option<String>,
}

#[napi(object, js_name = "CreateComponentIRNode")]
pub struct CreateComponentIRNode {
  #[napi(ts_type = "IRNodeTypes.CREATE_COMPONENT_NODE")]
  pub _type: IRNodeTypes,
  pub id: i32,
  pub tag: String,
  pub props: Vec<IRProps>,
  pub slots: Vec<IRSlots>,
  pub asset: bool,
  pub root: bool,
  pub once: bool,
  pub dynamic: Option<SimpleExpressionNode>,
  pub parent: Option<i32>,
  pub anchor: Option<i32>,
}

#[napi(object, js_name = "DeclareOldRefIRNode")]
pub struct DeclareOldRefIRNode {
  #[napi(ts_type = "IRNodeTypes.DECLARE_OLD_REF")]
  pub _type: IRNodeTypes,
  pub id: i32,
}

#[napi(object, js_name = "GetTextChildIRNode")]
pub struct GetTextChildIRNode {
  #[napi(ts_type = "IRNodeTypes.GET_TEXT_CHILD")]
  pub _type: IRNodeTypes,
  pub parent: i32,
}

#[napi(object, js_name = "SlotOutletIRNode")]
pub struct SlotOutletIRNode {
  #[napi(ts_type = "IRNodeTypes.SLOT_OUTLET_NODE")]
  pub _type: IRNodeTypes,
}

#[napi]
pub type OperationNode = Either18<
  IfIRNode,
  ForIRNode,
  SetTextIRNode,
  SetPropIRNode,
  SetDynamicPropsIRNode,
  SetDynamicEventsIRNode,
  SetNodesIRNode,
  SetEventIRNode,
  SetHtmlIRNode,
  SetTemplateRefIRNode,
  CreateNodesIRNode,
  InsertNodeIRNode,
  PrependNodeIRNode,
  DirectiveIRNode,
  CreateComponentIRNode,
  DeclareOldRefIRNode,
  SlotOutletIRNode,
  GetTextChildIRNode,
>;

#[napi]
pub type IRNode = Either<OperationNode, RootIRNode>;

#[napi]
pub enum DynamicFlag {
  NONE = 0,
  // This node is referenced and needs to be saved as a variable.
  REFERENCED = 1,
  // This node is not generated from template, but is generated dynamically.
  NON_TEMPLATE = 2,
  // This node needs to be inserted back into the template.
  INSERT = 4,
}

#[napi(object, js_name = "IRDynamicInfo")]
pub struct IRDynamicInfo {
  pub id: Option<i32>,
  pub flags: DynamicFlag,
  pub anchor: Option<i32>,
  pub children: Vec<IRDynamicInfo>,
  pub template: Option<i32>,
  pub has_dynamic_child: Option<bool>,
  #[napi(ts_type = "OperationNode")]
  pub operation: Option<MyBox<OperationNode>>,
}

#[napi(object, js_name = "IREffect")]
pub struct IREffect {
  pub expressions: Vec<SimpleExpressionNode>,
  pub operations: Vec<OperationNode>,
}

#[napi]
pub type SourceLocation = (i32, i32);

#[napi(object)]
pub struct SimpleExpressionNode {
  pub content: String,
  pub is_static: bool,
  pub loc: Option<SourceLocation>,
  #[napi(ts_type = "import('oxc-parser').Node")]
  pub ast: Option<Object<'static>>,
}

#[napi(object)]
pub struct DirectiveNode {
  // the normalized name without prefix or shorthands, e.g. "bind", "on"
  pub name: String,
  pub exp: Option<SimpleExpressionNode>,
  pub arg: Option<SimpleExpressionNode>,
  pub modifiers: Vec<SimpleExpressionNode>,
  pub loc: Option<SourceLocation>,
}

#[napi]
pub type InsertionStateTypes = Either3<IfIRNode, ForIRNode, CreateComponentIRNode>;

#[napi[ts_return_type = "op is InsertionStateTypes"]]
pub fn is_block_operation(#[napi(ts_arg_type = "OperationNode")] op: Object) -> bool {
  let _type = op.get::<IRNodeTypes>("type").ok().flatten();
  match _type {
    Some(IRNodeTypes::CREATE_COMPONENT_NODE) => true,
    Some(IRNodeTypes::IF) => true,
    Some(IRNodeTypes::FOR) => true,
    _ => false,
  }
}

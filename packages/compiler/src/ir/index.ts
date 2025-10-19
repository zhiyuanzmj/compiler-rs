import type {
  BlockIRNode,
  DynamicFlag,
  IRNodeTypes,
  OperationNode,
} from '@vue-jsx-vapor/compiler-rs'

export {
  CreateNodesIRNode,
  DynamicFlag,
  IRNode,
  IRNodeTypes,
  isBlockOperation,
  type BaseIRNode,
  type BlockIRNode,
  type CreateComponentIRNode,
  type DeclareOldRefIRNode,
  type DirectiveIRNode,
  type DirectiveNode,
  type ForIRNode,
  type GetTextChildIRNode,
  type IfIRNode,
  type InsertionStateTypes,
  type InsertNodeIRNode,
  type IREffect,
  type IRFor,
  type OperationNode,
  type PrependNodeIRNode,
  type SetDynamicEventsIRNode,
  type SetDynamicPropsIRNode,
  type SetEventIRNode,
  type SetHtmlIRNode,
  type SetNodesIRNode,
  type SetPropIRNode,
  type SetTemplateRefIRNode,
  type SetTextIRNode,
  type SimpleExpressionNode,
  type SourceLocation,
} from '@vue-jsx-vapor/compiler-rs'

export * from './component'

export interface IRDynamicInfo {
  id?: number
  flags: DynamicFlag
  anchor?: number
  children: Array<IRDynamicInfo>
  template?: number
  hasDynamicChild?: boolean
  operation?: OperationNode
  parent?: IRDynamicInfo
}

export interface RootIRNode {
  type: IRNodeTypes.ROOT
  node: object
  source: string
  templates: string[]
  rootTemplateIndex?: number
  component: Set<string>
  directive: Set<string>
  block: BlockIRNode
  hasTemplateRef: boolean
}

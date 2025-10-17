import {
  transformNode,
  type DirectiveTransformResult,
} from '@vue-jsx-vapor/compiler-rs'
import { extend, isArray, isString, NOOP } from '@vue/shared'
import {
  DynamicFlag,
  IRNodeTypes,
  type BlockIRNode,
  type IRDynamicInfo,
  type IRSlots,
  type OperationNode,
  type RootIRNode,
  type RootNode,
  type SimpleExpressionNode,
} from './ir'
import {
  findProp,
  getText,
  isConstantExpression,
  isConstantNode,
  isTemplate,
  newBlock,
  newDynamic,
  type CompilerError,
} from './utils'
import type { CodegenOptions } from './generate'
import type { JSXAttribute, JSXElement, JSXFragment } from 'oxc-parser'

export { DirectiveTransformResult, transformNode }

export type NodeTransform = (
  node: BlockIRNode['node'],
  context: TransformContext<BlockIRNode['node']>,
) =>
  | void
  | null
  | ((context: TransformContext) => void | null)
  | ((context: TransformContext) => void | null)[]

export type DirectiveTransform = (
  dir: JSXAttribute,
  node: JSXElement,
  context: TransformContext<JSXElement>,
) => DirectiveTransformResult | void | null

export type TransformOptions = CodegenOptions & {
  source?: string
  /**
   * Whether to compile components to createComponentWithFallback.
   * @default false
   */
  withFallback?: boolean
  /**
   * Indicates that transforms and codegen should try to output valid TS code
   */
  isTS?: boolean
  /**
   * An array of node transforms to be applied to every AST node.
   */
  nodeTransforms?: NodeTransform[]
  /**
   * An object of { name: transform } to be applied to every directive attribute
   * node found on element nodes.
   */
  directiveTransforms?: Record<string, DirectiveTransform | undefined>
  /**
   * Separate option for end users to extend the native elements list
   */
  isCustomElement?: (tag: string) => boolean | void
  onError?: (error: CompilerError) => void
}
const defaultOptions: Required<TransformOptions> = {
  source: '',
  sourceMap: false,
  filename: 'index.jsx',
  nodeTransforms: [],
  directiveTransforms: {},
  templates: [],
  isCustomElement: (tag: string) => !tag,
  isTS: false,
  withFallback: false,
  onError: (error: CompilerError) => {
    throw error
  },
}

export class TransformContext<
  T extends BlockIRNode['node'] = BlockIRNode['node'],
> {
  parent: TransformContext<RootNode | JSXElement | JSXFragment> | null = null
  root: TransformContext<RootNode>
  index: number = 0

  block: BlockIRNode
  options: Required<TransformOptions>

  template: string = ''
  childrenTemplate: (string | null)[] = []
  dynamic: IRDynamicInfo

  inVOnce: boolean = false
  inVFor: number = 0

  slots: IRSlots[] = []

  exitKey = 0
  exitBlocks = {}
  blocks = {}
  nodes = {}
  seen = new Set()

  private globalId = 0

  constructor(
    public ir: RootIRNode,
    public node: T,
    options: TransformOptions = {},
  ) {
    this.options = extend({}, defaultOptions, options)
    this.block = this.ir.block
    this.dynamic = this.ir.block.dynamic
    this.root = this as TransformContext<RootNode>
  }

  enterBlock(
    ir: BlockIRNode,
    isVFor: boolean = false,
    // should removed
    exclude_slots = false,
  ): [BlockIRNode, () => void] {
    const { block, template, dynamic, childrenTemplate, slots } = this
    this.block = ir
    this.dynamic = ir.dynamic
    this.template = ''
    this.childrenTemplate = []
    if (!exclude_slots) this.slots = []

    isVFor && this.inVFor++
    const exitBlock = () => {
      // exit
      this.registerTemplate()
      this.block = block
      this.template = template
      this.dynamic = dynamic
      this.childrenTemplate = childrenTemplate
      if (!exclude_slots) this.slots = slots
      isVFor && this.inVFor--
    }
    // @ts-expect-error should removed
    this.exitBlocks[this.exitKey] = exitBlock
    // @ts-expect-error should removed
    this.blocks[this.exitKey] = ir
    this.exitKey++
    return [ir, exitBlock]
  }

  increaseId = () => this.globalId++
  reference() {
    if (this.dynamic.id !== undefined) return this.dynamic.id
    this.dynamic.flags |= DynamicFlag.REFERENCED
    return (this.dynamic.id = this.increaseId())
  }

  pushTemplate(content: string) {
    const existing = this.ir.templates.indexOf(content)
    if (existing !== -1) return existing
    this.ir.templates.push(content)
    return this.ir.templates.length - 1
  }

  registerTemplate() {
    if (!this.template) return -1
    const id = this.pushTemplate(this.template)
    return (this.dynamic.template = id)
  }

  registerEffect(
    expressions: SimpleExpressionNode[] | boolean,
    operation: OperationNode,
    getEffectIndex = (): number => this.block.effect.length,
    getOperationIndex = (): number => this.block.operation.length,
  ) {
    if (expressions === true) {
      return this.registerOperation(operation, getOperationIndex)
    } else if (expressions !== false) {
      expressions = expressions.filter((exp) => !isConstantExpression(exp))
      if (
        this.inVOnce ||
        expressions.length === 0 ||
        expressions.every((e) => e.ast && isConstantNode(e.ast))
      ) {
        return this.registerOperation(operation, getOperationIndex)
      }
    }

    this.block.effect.splice(getEffectIndex(), 0, {
      expressions: [],
      operations: [operation],
    })
  }

  registerOperation(
    operation: OperationNode,
    getOperationIndex = (): number => this.block.operation.length,
  ) {
    this.block.operation.splice(getOperationIndex(), 0, operation)
  }

  create<E extends T>(node: E, index: number): TransformContext<E> {
    return Object.assign(Object.create(TransformContext.prototype), this, {
      node,
      parent: this as any,
      index,

      template: '',
      childrenTemplate: [],
      dynamic: newDynamic(),
    } satisfies Partial<TransformContext<T>>)
  }
}

// AST -> IR
export function transform(
  node: RootNode,
  options: TransformOptions = {},
): RootIRNode {
  const ir: RootIRNode = {
    type: IRNodeTypes.ROOT,
    node,
    source: node.source,
    templates: options.templates || [],
    component: new Set(),
    directive: new Set(),
    block: newBlock(node),
    hasTemplateRef: false,
  }

  const context = new TransformContext(ir, node, options)

  transformNode(context)

  return ir
}

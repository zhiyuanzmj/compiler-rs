import {
  registerTemplate,
  transformNode,
  type DirectiveTransformResult,
} from '@vue-jsx-vapor/compiler-rs'
import { extend } from '@vue/shared'
import {
  IRNodeTypes,
  type BlockIRNode,
  type IRSlots,
  type RootIRNode,
} from './ir'
import { newBlock, newDynamic, type CompilerError } from './utils'
import type { CodegenOptions } from './generate'

export { DirectiveTransformResult, transformNode }

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
   * Separate option for end users to extend the native elements list
   */
  isCustomElement?: (tag: string) => boolean | void
  onError?: (error: CompilerError) => void
}
const defaultOptions: Required<TransformOptions> = {
  source: '',
  sourceMap: false,
  filename: 'index.jsx',
  templates: [],
  isCustomElement: (tag: string) => !tag,
  isTS: false,
  withFallback: false,
  onError: (error: CompilerError) => {
    throw error
  },
}

export class TransformContext {
  parent: TransformContext | null = null
  root: TransformContext
  index: number = 0

  block: BlockIRNode
  options: Required<TransformOptions>

  template: string = ''
  childrenTemplate: (string | null)[] = []

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
    public node: Node,
    options: TransformOptions = {},
  ) {
    this.options = extend({}, defaultOptions, options)
    this.block = this.ir.block
    this.root = this as TransformContext
  }

  createBlock(node: Node) {
    return {
      type: IRNodeTypes.BLOCK,
      node,
      dynamic: newDynamic(),
      tempId: 0,
      effect: [],
      operation: [],
      returns: [],
    }
  }

  enterBlock(
    ir: BlockIRNode,
    isVFor: boolean = false,
    // should removed
    exclude_slots = false,
  ): [BlockIRNode, () => void] {
    const { block, template, childrenTemplate, slots } = this
    this.block = ir
    this.template = ''
    this.childrenTemplate = []
    if (!exclude_slots) this.slots = []

    isVFor && this.inVFor++
    const exitBlock = () => {
      // exit
      registerTemplate(this)
      this.block = block
      this.template = template
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

  create(node: Node, index: number): TransformContext {
    this.block.dynamic = newDynamic()
    return Object.assign(Object.create(TransformContext.prototype), this, {
      // block: this.block,
      // increaseId: this.increaseId,
      // blocks: this.blocks,
      // create: this.create,
      // enterBlock: this.enterBlock,
      // exitBlocks: this.exitBlocks,
      // exitKey: this.exitKey,
      // inVFor: this.inVFor,
      // inVOnce: this.inVOnce,
      // ir: this.ir,
      // nodes: this.nodes,
      // options: this.options,
      // root: this.root,
      // seen: this.seen,
      // slots: this.slots,

      node,
      parent: this as any,
      index,

      template: '',
      childrenTemplate: [],
    } satisfies Partial<TransformContext>)
  }
}

// AST -> IR
export function transform(
  node: Node,
  options: TransformOptions = {},
): RootIRNode {
  const ir: RootIRNode = {
    type: IRNodeTypes.ROOT,
    node,
    source: options.source || '',
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

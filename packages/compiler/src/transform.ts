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
import { newBlock, type CompilerError } from './utils'
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

import {
  jsxClosingFragment,
  jsxExpressionContainer,
  jsxFragment,
  jsxOpeningFragment,
  type Expression,
  type JSXElement,
  type JSXFragment,
  type Node,
} from '@babel/types'
import { createSimpleExpression } from '@vue/compiler-dom'
import {
  DynamicFlag,
  IRNodeTypes,
  type BlockIRNode,
  type IRDynamicInfo,
  type RootNode,
} from '../ir/index'
import { isTemplate } from '../utils'
import type { TransformContext } from '../transform'

export function newDynamic(): IRDynamicInfo {
  return {
    flags: DynamicFlag.REFERENCED,
    children: [],
  }
}

export function newBlock(node: BlockIRNode['node']): BlockIRNode {
  return {
    type: 1 satisfies IRNodeTypes.BLOCK,
    node,
    dynamic: newDynamic(),
    effect: [],
    operation: [],
    returns: [],
    tempId: 0,
  }
}

export function createBranch(
  node: Parameters<typeof wrapFragment>[0],
  context: TransformContext,
  isVFor?: boolean,
): [BlockIRNode, () => void] {
  context.node = node = wrapFragment(node)
  const branch: BlockIRNode = newBlock(node)
  const exitBlock = context.enterBlock(branch, isVFor)
  context.reference()
  return [branch, exitBlock]
}

export function wrapFragment(node: JSXElement | JSXFragment | Expression) {
  if (node.type === 'JSXFragment' || isTemplate(node)) {
    return node
  }

  return jsxFragment(jsxOpeningFragment(), jsxClosingFragment(), [
    node.type === 'JSXElement' ? node : jsxExpressionContainer(node),
  ])
}

export const EMPTY_EXPRESSION = createSimpleExpression('', true)

export const isFragmentNode = (
  node: Node | RootNode,
): node is JSXElement | JSXFragment | RootNode =>
  node.type === IRNodeTypes.ROOT ||
  node.type === 'JSXFragment' ||
  (node.type === 'JSXElement' && !!isTemplate(node))

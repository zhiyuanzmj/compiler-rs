import {
  DynamicFlag,
  type BlockIRNode,
  type IRDynamicInfo,
  type IRNodeTypes,
} from '../ir/index'
import { isTemplate } from '../utils'
import type { TransformContext } from '../transform'
import type { Expression, JSXElement, JSXFragment } from 'oxc-parser'

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

export function wrapFragment(
  node: JSXElement | JSXFragment | Expression,
): JSXFragment | JSXElement {
  if (node.type === 'JSXFragment' || isTemplate(node)) {
    return node as JSXFragment
  }

  return {
    type: 'JSXFragment',
    start: 0,
    end: 0,
    openingFragment: { type: 'JSXOpeningFragment', start: 0, end: 0 },
    closingFragment: { type: 'JSXClosingFragment', start: 0, end: 0 },
    children: [
      node.type === 'JSXElement'
        ? node
        : {
            type: 'JSXExpressionContainer',
            start: 0,
            end: 0,
            expression: node,
          },
    ],
  }
}

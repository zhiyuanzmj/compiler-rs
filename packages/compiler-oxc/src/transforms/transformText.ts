import {
  DynamicFlag,
  IRNodeTypes,
  type BlockIRNode,
  type RootNode,
} from '../ir'
import {
  findProp,
  getExpression,
  getLiteralExpressionValue,
  isEmptyText,
  isFragmentNode,
  isJSXComponent,
  isTemplate,
  resolveExpression,
  resolveJSXText,
} from '../utils'
import type { NodeTransform, TransformContext } from '../transform'
import {
  processConditionalExpression,
  processLogicalExpression,
} from './expression'
import type { JSXExpressionContainer, JSXText, Node } from 'oxc-parser'

type TextLike = JSXText | JSXExpressionContainer
const seen = new WeakMap<
  TransformContext<RootNode>,
  WeakSet<TextLike | BlockIRNode['node'] | RootNode>
>()

export function markNonTemplate(node: Node, context: TransformContext): void {
  seen.get(context.root)!.add(node)
}

export const transformText: NodeTransform = (node, context) => {
  if (!seen.has(context.root)) seen.set(context.root, new WeakSet())
  if (seen.get(context.root)!.has(node)) {
    context.dynamic.flags |= DynamicFlag.NON_TEMPLATE
    return
  }

  const isFragment = isFragmentNode(node)
  if (
    ((node.type === 'JSXElement' &&
      !isTemplate(node) &&
      !isJSXComponent(node)) ||
      isFragment) &&
    node.children.length
  ) {
    let hasInterp = false
    let isAllTextLike = true
    for (const c of node.children) {
      let expression
      if (
        c.type === 'JSXExpressionContainer' &&
        (expression = getExpression(c)) &&
        expression.type !== 'ConditionalExpression' &&
        expression.type !== 'LogicalExpression'
      ) {
        hasInterp = true
      } else if (c.type !== 'JSXText') {
        isAllTextLike = false
      }
    }
    // all text like with interpolation
    if (!isFragment && isAllTextLike && hasInterp) {
      processTextContainer((node as any).children, context)
    } else if (hasInterp) {
      // check if there's any text before interpolation, it needs to be merged
      for (let i = 0; i < node.children.length; i++) {
        const c = node.children[i]
        const prev = node.children[i - 1]
        if (
          c.type === 'JSXExpressionContainer' &&
          prev &&
          prev.type === 'JSXText'
        ) {
          // mark leading text node for skipping
          markNonTemplate(prev, context)
        }
      }
    }
  } else if (node.type === 'JSXExpressionContainer') {
    const expression = getExpression(node)
    if (expression.type === 'ConditionalExpression') {
      return processConditionalExpression(expression, context)
    } else if (expression.type === 'LogicalExpression') {
      return processLogicalExpression(expression, context)
    } else {
      processInterpolation(context)
    }
  } else if (node.type === 'JSXText') {
    const value = resolveJSXText(node)
    if (value) {
      context.template += value
    } else {
      context.dynamic.flags |= DynamicFlag.NON_TEMPLATE
    }
  }
}

function processInterpolation(context: TransformContext) {
  const parent = context.parent!.node
  const children = parent.children
  const nexts = children.slice(context.index)
  const idx = nexts.findIndex((n) => !isTextLike(n))
  const nodes = (idx !== -1 ? nexts.slice(0, idx) : nexts) as Array<TextLike>

  // merge leading text
  const prev = children[context.index - 1]
  if (prev && prev.type === 'JSXText') {
    nodes.unshift(prev)
  }

  const values = createTextLikeExpressions(nodes, context)
  if (!values.length) {
    context.dynamic.flags |= DynamicFlag.NON_TEMPLATE
    return
  }

  const id = context.reference()

  if (isFragmentNode(parent) || findProp(parent, 'v-slot')) {
    context.registerOperation({
      type: IRNodeTypes.CREATE_NODES,
      id,
      once: context.inVOnce,
      values,
    })
  } else {
    context.template += ' '
    context.registerOperation({
      type: IRNodeTypes.SET_NODES,
      element: id,
      once: context.inVOnce,
      values,
    })
  }
}

function processTextContainer(children: TextLike[], context: TransformContext) {
  const values = createTextLikeExpressions(children, context)
  const literals = values.map(getLiteralExpressionValue)
  if (literals.every((l) => l != null)) {
    context.childrenTemplate = literals
  } else {
    context.childrenTemplate = [' ']
    context.registerOperation({
      type: IRNodeTypes.GET_TEXT_CHILD,
      parent: context.reference(),
    })
    context.registerOperation({
      type: IRNodeTypes.SET_NODES,
      element: context.reference(),
      once: context.inVOnce,
      values,
      // indicates this node is generated, so prefix should be "x" instead of "n"
      generated: true,
    })
  }
}

function createTextLikeExpressions(
  nodes: TextLike[],
  context: TransformContext,
) {
  const values = []
  for (const node of nodes) {
    markNonTemplate(node, context)
    if (isEmptyText(node)) continue
    values.push(resolveExpression(node, context))
  }
  return values
}

function isTextLike(node: Node): node is TextLike {
  if (node.type === 'JSXExpressionContainer') {
    const expression = getExpression(node)
    return (
      expression.type !== 'ConditionalExpression' &&
      expression.type !== 'LogicalExpression'
    )
  } else {
    return node.type === 'JSXText'
  }
}

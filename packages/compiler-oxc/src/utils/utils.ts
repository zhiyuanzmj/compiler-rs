import { isBigIntLiteral, isNumericLiteral, isStringLiteral } from './check'
import type { Expression, Node } from 'oxc-parser'

export const TS_NODE_TYPES = [
  'TSAsExpression', // foo as number
  'TSTypeAssertion', // (<number>foo)
  'TSNonNullExpression', // foo!
  'TSInstantiationExpression', // foo<string>
  'TSSatisfiesExpression', // foo satisfies T
]

export function unwrapTSNode(node: Node): Node {
  if (TS_NODE_TYPES.includes(node.type)) {
    return unwrapTSNode((node as any).expression)
  } else {
    return node
  }
}

export function findProp(
  expression: Expression | undefined,
  key: string | RegExp,
) {
  if (expression?.type === 'JSXElement') {
    for (const attr of expression.openingElement.attributes) {
      const name =
        attr.type === 'JSXAttribute' &&
        (attr.name.type === 'JSXIdentifier'
          ? attr.name.name
          : attr.name.type === 'JSXNamespacedName'
            ? attr.name.namespace.name
            : ''
        ).split('_')[0]
      if (name && (typeof key === 'string' ? name === key : key.test(name))) {
        return attr
      }
    }
  }
}

export function getExpression(node: Node) {
  node = node.type === 'JSXExpressionContainer' ? node.expression : node
  return node.type === 'ParenthesizedExpression' ? node.expression : node
}

export function getTextLikeValue(node: Node, excludeNumber?: boolean) {
  node = node.type === 'JSXExpressionContainer' ? getExpression(node) : node
  if (isStringLiteral(node)) {
    return node.value
  } else if (
    !excludeNumber &&
    (isNumericLiteral(node) || isBigIntLiteral(node))
  ) {
    return String(node.value)
  } else if (node.type === 'TemplateLiteral' && node.expressions.length === 0) {
    return node.quasis[0].value.cooked!
  }
}

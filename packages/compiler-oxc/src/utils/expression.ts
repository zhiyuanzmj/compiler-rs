import { isGloballyAllowed, makeMap } from '@vue/shared'
import {
  parseSync,
  type ExpressionStatement,
  type JSXAttribute,
  type Node,
} from 'oxc-parser'
import type { SourceLocation } from '../ir'
import type { TransformContext } from '../transform'
import { isStringLiteral } from './check'
import { resolveJSXText } from './text'
import { getExpression, getTextLikeValue, unwrapTSNode } from './utils'

export interface SimpleExpressionNode {
  content: string
  isStatic: boolean
  loc: SourceLocation | null | undefined
  ast?: Node
}

export const locStub: SourceLocation = [0, 0]
export function createSimpleExpression(
  content: string,
  isStatic: boolean = false,
  ast?: Node,
  loc?: SourceLocation,
): SimpleExpressionNode {
  return {
    content,
    isStatic,
    ast,
    loc: loc || (ast ? ast.range : locStub),
  }
}

export const isLiteralWhitelisted: (key: string) => boolean =
  /*@__PURE__*/ makeMap('true,false,null,this')
export function isConstantExpression(exp: SimpleExpressionNode) {
  return (
    isLiteralWhitelisted(exp.content) ||
    isGloballyAllowed(exp.content) ||
    getLiteralExpressionValue(exp) !== null
  )
}

export function getLiteralExpressionValue(
  exp: SimpleExpressionNode,
): string | null {
  if (exp.ast) {
    const res = getTextLikeValue(exp.ast)
    if (res != null) {
      return res
    }
  }
  return exp.isStatic ? exp.content : null
}

export const EMPTY_EXPRESSION = createSimpleExpression('', true)

export function resolveExpression(
  node: Node | undefined | null,
  context: TransformContext,
): SimpleExpressionNode {
  if (!node) return EMPTY_EXPRESSION
  node = unwrapTSNode(getExpression(node))
  const isStatic =
    isStringLiteral(node) ||
    node.type === 'JSXText' ||
    node.type === 'JSXIdentifier'
  const source =
    node.type === 'JSXIdentifier'
      ? node.name
      : isStringLiteral(node)
        ? node.value
        : node.type === 'JSXText'
          ? resolveJSXText(node)
          : node.type === 'Identifier'
            ? node.name
            : context.ir.source.slice(node.start!, node.end!)
  return createSimpleExpression(source, isStatic, node)
}

export function propToExpression(
  prop: JSXAttribute,
  context: TransformContext,
) {
  return prop.type === 'JSXAttribute' &&
    prop.value?.type === 'JSXExpressionContainer'
    ? resolveExpression(prop.value.expression, context)
    : EMPTY_EXPRESSION
}

export function parseExpression(filename: string, source: string) {
  return (
    parseSync(filename, source, {
      // @ts-ignore
      experimentalRawTransfer: true,
    }).program.body[0] as ExpressionStatement
  ).expression
}

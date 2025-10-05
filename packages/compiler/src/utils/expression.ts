import {
  createSimpleExpression,
  EMPTY_EXPRESSION,
  locStub,
  resolveExpression,
} from '@vue-jsx-vapor/compiler-rs'
import { isGloballyAllowed, makeMap } from '@vue/shared'
import {
  parseSync,
  type ExpressionStatement,
  type JSXAttribute,
  type Node,
} from 'oxc-parser'
import type { SimpleExpressionNode, SourceLocation } from '../ir'
import type { TransformContext } from '../transform'
import { getTextLikeValue } from './utils'

export { createSimpleExpression, EMPTY_EXPRESSION, locStub, resolveExpression }

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
  return (parseSync(filename, source).program.body[0] as ExpressionStatement)
    .expression
}

import { parseSync, type ExpressionStatement } from 'oxc-parser'
export {
  createSimpleExpression,
  EMPTY_EXPRESSION,
  getLiteralExpressionValue,
  isConstantExpression,
  locStub,
  resolveExpression,
} from '@vue-jsx-vapor/compiler-rs'

export function parseExpression(filename: string, source: string) {
  return (parseSync(filename, source).program.body[0] as ExpressionStatement)
    .expression
}

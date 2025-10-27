import { genExpression } from '@vue-jsx-vapor/compiler-rs'
import { IRNodeTypes, type IfIRNode } from '../ir'
import {
  buildCodeFragment,
  genCall,
  NEWLINE,
  type CodeFragment,
} from '../utils'
import type { CodegenContext } from '../generate'
import { genBlock } from './block'

export function genIf(
  oper: IfIRNode,
  context: CodegenContext,
  isNested = false,
): CodeFragment[] {
  const { helper } = context
  const { condition, positive, negative, once } = oper
  const [frag, push] = buildCodeFragment()

  const conditionExpr: CodeFragment[] = [
    '() => (',
    ...genExpression(condition, context),
    ')',
  ]

  const positiveArg = genBlock(positive, context)
  let negativeArg: CodeFragment[] = null

  if (negative) {
    if (negative.type === IRNodeTypes.BLOCK) {
      negativeArg = genBlock(negative, context)
    } else {
      negativeArg = ['() => ', ...genIf(negative as IfIRNode, context, true)]
    }
  }

  if (!isNested) push(NEWLINE, `const n${oper.id} = `)
  push(
    ...genCall(helper('createIf'), [
      conditionExpr,
      positiveArg,
      negativeArg,
      once ? 'true' : null,
    ]),
  )

  return frag
}

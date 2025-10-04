import { genCall, NEWLINE, type CodeFragment } from '../utils'
import type { CodegenContext } from '../generate'
import type { DirectiveIRNode, SimpleExpressionNode } from '../ir'
import { genExpression } from './expression'

const helperMap = {
  text: 'applyTextModel',
  radio: 'applyRadioModel',
  checkbox: 'applyCheckboxModel',
  select: 'applySelectModel',
  dynamic: 'applyDynamicModel',
} as const

// This is only for built-in v-model on native elements.
export function genVModel(
  oper: DirectiveIRNode,
  context: CodegenContext,
): CodeFragment[] {
  const {
    modelType,
    element,
    dir: { exp, modifiers },
  } = oper

  return [
    NEWLINE,
    ...genCall(
      context.helper(helperMap[modelType!]),
      `n${element}`,
      // getter
      [`() => (`, ...genExpression(exp!, context), `)`],
      // setter
      genModelHandler(exp!, context),
      // modifiers
      modifiers.length
        ? `{ ${modifiers.map((e) => `${e.content}: true`).join(',')} }`
        : undefined,
    ),
  ]
}

export function genModelHandler(
  exp: SimpleExpressionNode,
  context: CodegenContext,
): CodeFragment[] {
  return ['_value => (', ...genExpression(exp, context, '_value'), ')']
}

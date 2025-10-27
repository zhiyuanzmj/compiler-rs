import { getDelimitersArray } from '@vue-jsx-vapor/compiler-rs'
import { IRNodeTypes, type DirectiveIRNode, type OperationNode } from '../ir'
import {
  createSimpleExpression,
  genCall,
  genMulti,
  isSimpleIdentifier,
  NEWLINE,
  toValidAssetId,
  type CodeFragment,
} from '../utils'
import type { CodegenContext } from '../generate'
import { genExpression } from './expression'
import { genVModel } from './vModel'
import { genVShow } from './vShow'

export function genBuiltinDirective(
  oper: DirectiveIRNode,
  context: CodegenContext,
): CodeFragment[] {
  switch (oper.name) {
    case 'show':
      return genVShow(oper, context)
    case 'model':
      return genVModel(oper, context)
    default:
      return []
  }
}

/**
 * user directives via `withVaporDirectives`
 * TODO the compiler side is implemented but no runtime support yet
 * it was removed due to perf issues
 */
export function genDirectivesForElement(
  id: number,
  context: CodegenContext,
): CodeFragment[] {
  const dirs = filterCustomDirectives(id, context.block.operation)
  return dirs.length ? genCustomDirectives(dirs, context) : []
}

function genCustomDirectives(
  opers: DirectiveIRNode[],
  context: CodegenContext,
): CodeFragment[] {
  const { helper } = context

  const element = `n${opers[0].element}`
  const directiveItems = opers.map(genDirectiveItem)
  const directives = genMulti(getDelimitersArray(), directiveItems)

  return [
    NEWLINE,
    ...genCall(helper('withVaporDirectives'), [element, directives]),
  ]

  function genDirectiveItem({
    dir,
    name,
    asset,
  }: DirectiveIRNode): CodeFragment[] {
    const directiveVar = asset
      ? toValidAssetId(name, 'directive')
      : genExpression(createSimpleExpression(name), context)
    const value = dir.exp && ['() => ', ...genExpression(dir.exp, context)]
    const argument = dir.arg && genExpression(dir.arg, context)
    const modifiers = dir.modifiers.length
      ? ['{ ', genDirectiveModifiers(dir.modifiers.map((m) => m.content)), ' }']
      : null

    return genMulti(
      ['[', ']', ', ', 'void 0'],
      [directiveVar, value, argument, modifiers],
    )
  }
}

export function genDirectiveModifiers(modifiers: string[]): string {
  return modifiers
    .map(
      (value) =>
        `${isSimpleIdentifier(value) ? value : JSON.stringify(value)}: true`,
    )
    .join(', ')
}

function filterCustomDirectives(
  id: number,
  operations: OperationNode[],
): DirectiveIRNode[] {
  return operations.filter(
    (oper): oper is DirectiveIRNode =>
      oper.type === IRNodeTypes.DIRECTIVE &&
      oper.element === id &&
      !oper.builtin,
  )
}

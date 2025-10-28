import { getDelimitersArray } from '@vue-jsx-vapor/compiler-rs'
import {
  buildCodeFragment,
  genCall,
  genMulti,
  INDENT_END,
  INDENT_START,
  NEWLINE,
  toValidAssetId,
  type CodeFragment,
} from '../utils'
import type { CodegenContext } from '../generate'
import type { BlockIRNode } from '../ir'
import { genEffects, genOperations } from './operation'
import { genChildren, genSelf } from './template'

export function genBlock(
  oper: BlockIRNode,
  context: CodegenContext,
  args: CodeFragment[] = [],
  root?: boolean,
): CodeFragment[] {
  return [
    '(',
    ...args,
    ') => {',
    INDENT_START,
    ...genBlockContent(oper, context, root),
    INDENT_END,
    NEWLINE,
    '}',
  ]
}

export function genBlockContent(
  block: BlockIRNode,
  context: CodegenContext,
  root?: boolean,
  genEffectsExtraFrag?: () => CodeFragment[],
): CodeFragment[] {
  const [frag, push] = buildCodeFragment()
  const resetBlock = context.enterBlock(block)
  const { dynamic, effect, operation, returns } = context.block

  if (root) {
    for (let name of context.ir.component) {
      const id = toValidAssetId(name, 'component')
      const maybeSelfReference = name.endsWith('__self')
      if (maybeSelfReference) name = name.slice(0, -6)
      push(
        NEWLINE,
        `const ${id} = `,
        ...genCall(context.helper('resolveComponent'), [
          JSON.stringify(name),
          // pass additional `maybeSelfReference` flag
          maybeSelfReference ? 'true' : null,
        ]),
      )
    }
    genResolveAssets('directive', 'resolveDirective')
  }

  for (const child of dynamic.children) {
    push(...genSelf(child, context))
  }
  for (const child of dynamic.children) {
    if (!child.hasDynamicChild) {
      push(...genChildren(child.children, context, push, `n${child.id!}`))
    }
  }

  push(...genOperations(operation, context))
  push(...genEffects(effect, context, genEffectsExtraFrag))

  push(NEWLINE, `return `)

  const returnNodes = returns.map((n) => `n${n}`)
  const returnsCode: CodeFragment[] =
    returnNodes.length > 1
      ? genMulti(getDelimitersArray(), returnNodes)
      : [returnNodes[0] || 'null']
  push(...returnsCode)

  resetBlock()
  return frag

  function genResolveAssets(kind: 'component' | 'directive', helper: string) {
    for (const name of context.ir[kind]) {
      push(
        NEWLINE,
        `const ${toValidAssetId(name, kind)} = `,
        ...genCall(context.helper(helper), [JSON.stringify(name)]),
      )
    }
  }
}

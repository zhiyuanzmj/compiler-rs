import { genDirectivesForElement } from '@vue-jsx-vapor/compiler-rs'
import { DynamicFlag, type IRDynamicInfo } from '../ir'
import {
  buildCodeFragment,
  genCall,
  NEWLINE,
  type CodeFragment,
} from '../utils'
import type { CodegenContext } from '../generate'
import { genOperationWithInsertionState } from './operation'
export { genTemplates } from '@vue-jsx-vapor/compiler-rs'

export function genSelf(
  dynamic: IRDynamicInfo,
  context: CodegenContext,
): CodeFragment[] {
  const [frag, push] = buildCodeFragment()
  const { id, template, operation } = dynamic

  if (id !== undefined && template !== undefined) {
    push(NEWLINE, `const n${id} = t${template}()`)
    push(...genDirectivesForElement(id, context))
  }

  if (operation) {
    push(...genOperationWithInsertionState(operation, context))
  }

  push(...genChildren(dynamic.children, context, push, `n${id}`))

  return frag
}

export function genChildren(
  children: IRDynamicInfo[],
  context: CodegenContext,
  pushBlock: (...items: CodeFragment[]) => number,
  from: string,
): CodeFragment[] {
  const { helper } = context
  const [frag, push] = buildCodeFragment()

  let offset = 0
  let prev: [variable: string, elementIndex: number] | undefined

  for (const [index, child] of children.entries()) {
    if (child.flags & DynamicFlag.NON_TEMPLATE) {
      offset--
    }

    const id =
      child.flags & DynamicFlag.REFERENCED
        ? child.flags & DynamicFlag.INSERT
          ? child.anchor
          : child.id
        : undefined

    if (id === undefined && !child.hasDynamicChild) {
      push(...genSelf(child, context))
      continue
    }

    const elementIndex = Number(index) + offset
    // p for "placeholder" variables that are meant for possible reuse by
    // other access paths
    const variable = id === undefined ? `p${context.block.tempId++}` : `n${id}`
    pushBlock(NEWLINE, `const ${variable} = `)

    if (prev) {
      if (elementIndex - prev[1] === 1) {
        pushBlock(...genCall(helper('next'), [prev[0]]))
      } else {
        pushBlock(...genCall(helper('nthChild'), [from, String(elementIndex)]))
      }
    } else if (elementIndex === 0) {
      pushBlock(...genCall(helper('child'), [from]))
    } else {
      // check if there's a node that we can reuse from
      let init = genCall(helper('child'), [from])
      if (elementIndex === 1) {
        init = genCall(helper('next'), [init])
      } else if (elementIndex > 1) {
        init = genCall(helper('nthChild'), [from, String(elementIndex)])
      }
      pushBlock(...init)
    }

    const child_children = child.children
    if (id === child.anchor && !child.hasDynamicChild) {
      push(...genSelf(child, context))
    }

    if (id !== undefined) {
      push(...genDirectivesForElement(id, context))
    }

    prev = [variable, elementIndex]
    push(...genChildren(child_children, context, pushBlock, variable))
  }

  return frag
}

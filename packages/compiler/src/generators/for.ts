import { extend, isGloballyAllowed } from '@vue/shared'
import {
  createSimpleExpression,
  genCall,
  genMulti,
  INDENT_END,
  INDENT_START,
  isConstantNode,
  isStringLiteral,
  NEWLINE,
  parseExpression,
  walkAST,
  walkIdentifiers,
  type CodeFragment,
  type SimpleExpressionNode,
} from '../utils'
import type { CodegenContext } from '../generate'
import type { BlockIRNode, ForIRNode, IREffect } from '../ir'
import { genBlockContent } from './block'
import { genExpression } from './expression'
import { genOperation } from './operation'
import type { Expression, IdentifierName, Node } from 'oxc-parser'

/**
 * Flags to optimize vapor `createFor` runtime behavior, shared between the
 * compiler and the runtime
 */
export enum VaporVForFlags {
  /**
   * v-for is the only child of a parent container, so it can take the fast
   * path with textContent = '' when the whole list is emptied
   */
  FAST_REMOVE = 1,
  /**
   * v-for used on component - we can skip creating child scopes for each block
   * because the component itself already has a scope.
   */
  IS_COMPONENT = 1 << 1,
  /**
   * v-for inside v-ince
   */
  ONCE = 1 << 2,
}

export function genFor(
  oper: ForIRNode,
  context: CodegenContext,
): CodeFragment[] {
  const { helper, ir } = context
  const {
    source,
    value,
    key,
    index,
    render,
    keyProp,
    once,
    id,
    component,
    onlyChild,
  } = oper

  let rawValue: string | null = null
  const rawKey = key && key.content
  const rawIndex = index && index.content

  const sourceExpr = ['() => (', ...genExpression(source, context), ')']
  const idToPathMap = parseValueDestructure()

  const [depth, exitScope] = context.enterScope()
  const idMap: Record<string, string | SimpleExpressionNode | null> = {}

  const itemVar = `_for_item${depth}`
  idMap[itemVar] = null

  idToPathMap.forEach((pathInfo, id) => {
    let path = `${itemVar}.value${pathInfo ? pathInfo.path : ''}`
    if (pathInfo) {
      if (pathInfo.helper) {
        idMap[pathInfo.helper] = null
        path = `${pathInfo.helper}(${path}, ${pathInfo.helperArgs})`
      }
      if (pathInfo.dynamic) {
        const node = (idMap[id] = createSimpleExpression(path))
        node.ast = parseExpression(context.options.filename, `(${path})`)
      } else {
        idMap[id] = path
      }
    } else {
      idMap[id] = path
    }
  })

  const args = [itemVar]
  if (rawKey) {
    const keyVar = `_for_key${depth}`
    args.push(`, ${keyVar}`)
    idMap[rawKey] = `${keyVar}.value`
    idMap[keyVar] = null
  }
  if (rawIndex) {
    const indexVar = `_for_index${depth}`
    args.push(`, ${indexVar}`)
    idMap[rawIndex] = `${indexVar}.value`
    idMap[indexVar] = null
  }

  const { selectorPatterns, keyOnlyBindingPatterns } = matchPatterns(
    render,
    keyProp,
    idMap,
    ir.source,
  )
  const selectorDeclarations: CodeFragment[] = []
  const selectorSetup: CodeFragment[] = []

  for (const [i, { selector }] of selectorPatterns.entries()) {
    const selectorName = `_selector${id}_${i}`
    selectorDeclarations.push(`let ${selectorName}`, NEWLINE)
    if (i === 0) {
      selectorSetup.push(`({ createSelector }) => {`, INDENT_START)
    }
    selectorSetup.push(
      NEWLINE,
      `${selectorName} = `,
      ...genCall(`createSelector`, [
        `() => `,
        ...genExpression(selector, context),
      ]),
    )
    if (i === selectorPatterns.length - 1) {
      selectorSetup.push(INDENT_END, NEWLINE, '}')
    }
  }

  const blockFn = context.withId(() => {
    const frag: CodeFragment[] = []
    frag.push('(', ...args, ') => {', INDENT_START)
    if (selectorPatterns.length || keyOnlyBindingPatterns.length) {
      frag.push(
        ...genBlockContent(render, context, false, () => {
          const patternFrag: CodeFragment[] = []

          for (const [i, { effect }] of selectorPatterns.entries()) {
            patternFrag.push(
              NEWLINE,
              `_selector${id}_${i}(() => {`,
              INDENT_START,
            )
            for (const oper of effect.operations) {
              patternFrag.push(...genOperation(oper, context))
            }
            patternFrag.push(INDENT_END, NEWLINE, `})`)
          }

          for (const { effect } of keyOnlyBindingPatterns) {
            for (const oper of effect.operations) {
              patternFrag.push(...genOperation(oper, context))
            }
          }

          return patternFrag
        }),
      )
    } else {
      frag.push(...genBlockContent(render, context))
    }
    frag.push(INDENT_END, NEWLINE, '}')
    return frag
  }, idMap)
  exitScope()

  let flags = 0
  if (onlyChild) {
    flags |= VaporVForFlags.FAST_REMOVE
  }
  if (component) {
    flags |= VaporVForFlags.IS_COMPONENT
  }
  if (once) {
    flags |= VaporVForFlags.ONCE
  }

  return [
    NEWLINE,
    ...selectorDeclarations,
    `const n${id} = `,
    ...genCall(
      [helper('createFor'), 'undefined'],
      sourceExpr,
      blockFn,
      genCallback(keyProp),
      flags ? String(flags) : undefined,
      selectorSetup.length ? selectorSetup : undefined,
      // todo: hydrationNode
    ),
  ]

  // construct a id -> accessor path map.
  // e.g. `{ x: { y: [z] }}` -> `Map{ 'z' => '.x.y[0]' }`
  function parseValueDestructure() {
    const map = new Map<
      string,
      {
        path: string
        dynamic: boolean
        helper?: string
        helperArgs?: string
      } | null
    >()
    if (value) {
      rawValue = value && value.content
      if (value.ast && value.ast.type !== 'Identifier') {
        walkIdentifiers(
          value.ast,
          (id, _, parentStack, isReference, isLocal) => {
            if (isReference && !isLocal) {
              let path = ''
              let isDynamic = false
              let helper
              let helperArgs
              for (let i = 0; i < parentStack.length; i++) {
                const parent = parentStack[i]
                const child = parentStack[i + 1] || id

                if (parent.type === 'Property' && parent.value === child) {
                  if (isStringLiteral(parent.key)) {
                    path += `[${JSON.stringify(parent.key.value)}]`
                  } else if (parent.computed) {
                    isDynamic = true
                    path += `[${value.content.slice(
                      parent.key.start! - 1,
                      parent.key.end! - 1,
                    )}]`
                  } else {
                    // non-computed, can only be identifier
                    path += `.${(parent.key as IdentifierName).name}`
                  }
                } else if (parent.type === 'ArrayExpression') {
                  const index = parent.elements.indexOf(child as any)
                  if (child.type === 'SpreadElement') {
                    path += `.slice(${index})`
                  } else {
                    path += `[${index}]`
                  }
                } else if (
                  parent.type === 'ObjectExpression' &&
                  child.type === 'SpreadElement'
                ) {
                  helper = context.helper('getRestElement')
                  helperArgs = `[${parent.properties
                    .filter((p) => p.type === 'Property')
                    .map((p) => {
                      if (isStringLiteral(p.key)) {
                        return JSON.stringify(p.key.value)
                      } else if (p.computed) {
                        isDynamic = true
                        return value.content.slice(
                          p.key.start! - 1,
                          p.key.end! - 1,
                        )
                      } else {
                        return JSON.stringify((p.key as IdentifierName).name)
                      }
                    })
                    .join(', ')}]`
                }
              }
              map.set(id.name, { path, dynamic: isDynamic, helper, helperArgs })
            }
          },
          true,
        )
      } else {
        map.set(rawValue, null)
      }
    }
    return map
  }

  function genCallback(expr: SimpleExpressionNode | undefined) {
    if (!expr) return false
    const res = context.withId(
      () => genExpression(expr, context),
      genSimpleIdMap(),
    )
    return [
      ...genMulti(
        ['(', ')', ', '],
        rawValue ? rawValue : rawKey || rawIndex ? '_' : undefined,
        rawKey ? rawKey : rawIndex ? '__' : undefined,
        rawIndex,
      ),
      ' => (',
      ...res,
      ')',
    ]
  }

  function genSimpleIdMap() {
    const idMap: Record<string, null> = {}
    if (rawKey) idMap[rawKey] = null
    if (rawIndex) idMap[rawIndex] = null
    idToPathMap.forEach((_, id) => (idMap[id] = null))
    return idMap
  }
}

function matchPatterns(
  render: BlockIRNode,
  keyProp: SimpleExpressionNode | undefined,
  idMap: Record<string, string | SimpleExpressionNode | null>,
  source: string,
) {
  const selectorPatterns: NonNullable<
    ReturnType<typeof matchSelectorPattern>
  >[] = []
  const keyOnlyBindingPatterns: NonNullable<
    ReturnType<typeof matchKeyOnlyBindingPattern>
  >[] = []

  render.effect = render.effect.filter((effect) => {
    if (keyProp != null) {
      const selector = matchSelectorPattern(
        effect,
        keyProp.content,
        idMap,
        source,
      )
      if (selector) {
        selectorPatterns.push(selector)
        return false
      }
      const keyOnly = matchKeyOnlyBindingPattern(
        effect,
        keyProp.content,
        source,
      )
      if (keyOnly) {
        keyOnlyBindingPatterns.push(keyOnly)
        return false
      }
    }

    return true
  })

  return {
    keyOnlyBindingPatterns,
    selectorPatterns,
  }
}

function matchKeyOnlyBindingPattern(
  effect: IREffect,
  key: string,
  source: string,
):
  | {
      effect: IREffect
    }
  | undefined {
  // TODO: expressions can be multiple?
  if (effect.expressions.length === 1) {
    const ast = effect.expressions[0].ast
    if (
      typeof ast === 'object' &&
      ast !== null &&
      isKeyOnlyBinding(ast, key, source)
    ) {
      return { effect }
    }
  }
}

function matchSelectorPattern(
  effect: IREffect,
  key: string,
  idMap: Record<string, string | SimpleExpressionNode | null>,
  source: string,
):
  | {
      effect: IREffect
      selector: SimpleExpressionNode
    }
  | undefined {
  // TODO: expressions can be multiple?
  if (effect.expressions.length === 1) {
    const ast = effect.expressions[0].ast
    if (!ast) return
    const offset = ast.start
    if (typeof ast === 'object') {
      const matcheds: [key: Expression, selector: Expression][] = []

      walkAST(ast, {
        enter(node: Node) {
          if (
            typeof node === 'object' &&
            node &&
            node.type === 'BinaryExpression' &&
            node.operator === '==='
          ) {
            const { left, right } = node
            for (const [a, b] of [
              [left, right],
              [right, left],
            ]) {
              const aIsKey = isKeyOnlyBinding(a, key, source)
              const bIsKey = isKeyOnlyBinding(b, key, source)
              const bVars = analyzeVariableScopes(b, idMap)
              if (aIsKey && !bIsKey && !bVars.locals.length) {
                matcheds.push([a, b])
              }
            }
          }
        },
      })

      if (matcheds.length === 1) {
        const [key, selector] = matcheds[0]
        const content = effect.expressions[0].content

        let hasExtraId = false
        const parentStackMap = new Map<IdentifierName, Node[]>()
        const parentStack: Node[] = []
        walkIdentifiers(
          ast,
          (id) => {
            if (id.start !== key.start && id.start !== selector.start) {
              hasExtraId = true
            }
            parentStackMap.set(id, parentStack.slice())
          },
          false,
          parentStack,
        )

        if (!hasExtraId) {
          const name = content.slice(
            selector.start! - offset,
            selector.end! - offset,
          )
          return {
            effect,
            selector: {
              content: name,
              ast: extend({}, selector, {
                start: 1,
                end: name.length + 1,
              }),
              loc: selector.range,
              isStatic: false,
            },
          }
        }
      }
    }

    const content = effect.expressions[0].content
    if (
      typeof ast === 'object' &&
      ast &&
      ast.type === 'ConditionalExpression' &&
      ast.test.type === 'BinaryExpression' &&
      ast.test.operator === '===' &&
      isConstantNode(ast.consequent) &&
      isConstantNode(ast.alternate)
    ) {
      const left = ast.test.left
      const right = ast.test.right
      for (const [a, b] of [
        [left, right],
        [right, left],
      ]) {
        const aIsKey = isKeyOnlyBinding(a, key, source)
        const bIsKey = isKeyOnlyBinding(b, key, source)
        const bVars = analyzeVariableScopes(b, idMap)
        if (aIsKey && !bIsKey && !bVars.locals.length) {
          return {
            effect,
            selector: {
              content: content.slice(b.start! - offset, b.end! - offset),
              ast: b,
              loc: b.range,
              isStatic: false,
            },
          }
        }
      }
    }
  }
}

function analyzeVariableScopes(
  ast: Node,
  idMap: Record<string, string | SimpleExpressionNode | null>,
) {
  const globals: string[] = []
  const locals: string[] = []

  const ids: IdentifierName[] = []
  const parentStackMap = new Map<IdentifierName, Node[]>()
  const parentStack: Node[] = []
  walkIdentifiers(
    ast,
    (id) => {
      ids.push(id)
      parentStackMap.set(id, parentStack.slice())
    },
    false,
    parentStack,
  )

  for (const id of ids) {
    if (isGloballyAllowed(id.name)) {
      continue
    }
    if (idMap[id.name]) {
      locals.push(id.name)
    } else {
      globals.push(id.name)
    }
  }

  return { globals, locals }
}

function isKeyOnlyBinding(expr: Node, key: string, source: string) {
  let only = true
  walkAST(expr, {
    enter(node) {
      if (source.slice(node.start, node.end) === key) {
        this.skip()
        return
      }
      if (node.type === 'Identifier') {
        only = false
      }
    },
  })
  return only
}

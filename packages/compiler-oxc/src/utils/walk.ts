import {
  isForStatement,
  isFunctionType,
  isIdentifier,
  isReferencedIdentifier,
} from './check'
import { extractIdentifiers } from './extract'
import { TS_NODE_TYPES } from './utils'

import type {
  BlockStatement,
  ForInStatement,
  ForOfStatement,
  ForStatement,
  Function,
  IdentifierName,
  Node,
  Program,
} from 'oxc-parser'

type WalkerContext<T = Node> = {
  skip: () => void
  remove: () => void
  replace: (node: T) => void
}

type SyncHandler<T = Node> = (
  this: WalkerContext<T>,
  node: T,
  parent: T | null,
  key: string | number | symbol | null | undefined,
  index: number | null | undefined,
) => void

export class SyncWalker<T = Node> {
  enter: SyncHandler<T>
  leave: SyncHandler<T>
  context: WalkerContext<T>
  constructor(enter: SyncHandler<T>, leave: SyncHandler<T>) {
    this.context = {
      skip: () => (this.should_skip = true),
      remove: () => (this.should_remove = true),
      replace: (node) => (this.replacement = node),
    }

    this.enter = enter

    this.leave = leave
  }

  replace(
    parent?: T | null,
    prop?: keyof T | null,
    index?: number | null,
    node?: T,
  ) {
    if (parent && prop) {
      if (index != null) {
        ;(parent[prop] as any)[index] = node!
      } else {
        ;(parent[prop] as any) = node
      }
    }
  }

  remove(parent?: T | null, prop?: keyof T | null, index?: number | null) {
    if (parent && prop) {
      if (index !== null && index !== undefined) {
        ;(parent[prop] as any).splice(index, 1)
      } else {
        delete parent[prop]
      }
    }
  }

  visit(
    node: T,
    parent: T | null,
    prop?: keyof T,
    index?: number | null,
  ): T | null {
    if (node) {
      if (this.enter) {
        const _should_skip = this.should_skip
        const _should_remove = this.should_remove
        const _replacement = this.replacement
        this.should_skip = false
        this.should_remove = false
        this.replacement = null

        this.enter.call(this.context, node, parent, prop, index)

        if (this.replacement) {
          node = this.replacement
          this.replace(parent, prop, index, node)
        }

        if (this.should_remove) {
          this.remove(parent, prop, index)
        }

        const skipped = this.should_skip
        const removed = this.should_remove

        this.should_skip = _should_skip
        this.should_remove = _should_remove
        this.replacement = _replacement

        if (skipped) return node
        if (removed) return null
      }

      let key: keyof T

      // eslint-disable-next-line no-restricted-syntax
      for (key in node) {
        const value = node[key]

        if (value && typeof value === 'object') {
          if (Array.isArray(value)) {
            const nodes = value
            for (let i = 0; i < nodes.length; i += 1) {
              const item = nodes[i]
              if (isNode(item) && !this.visit(item as any, node, key, i)) {
                // removed
                i--
              }
            }
          } else if (isNode(value)) {
            this.visit(value as any, node, key, null)
          }
        }
      }

      if (this.leave) {
        const _replacement = this.replacement
        const _should_remove = this.should_remove
        this.replacement = null
        this.should_remove = false

        this.leave.call(this.context, node, parent, prop, index)

        if (this.replacement) {
          node = this.replacement
          this.replace(parent, prop, index, node)
        }

        if (this.should_remove) {
          this.remove(parent, prop, index)
        }

        const removed = this.should_remove

        this.replacement = _replacement
        this.should_remove = _should_remove

        if (removed) return null
      }
    }

    return node
  }
  should_skip = false
  should_remove = false
  replacement: T | null = null
}

function isNode<T = Node>(value: any): value is T {
  return (
    value !== null &&
    typeof value === 'object' &&
    'type' in value &&
    typeof value.type === 'string'
  )
}

interface WalkThis<T> {
  skip: () => void
  remove: () => void
  replace: (node: T) => void
}

type WalkCallback<T, R> = (
  this: WalkThis<T>,
  node: T,
  parent: T | null | undefined,
  key: string | null | undefined,
  index: number | null | undefined,
) => R

interface WalkHandlers<T, R> {
  enter?: WalkCallback<T, R>
  leave?: WalkCallback<T, R>
}

export function walk<T>(
  ast: T,
  { enter, leave }: { enter: SyncHandler<T>; leave: SyncHandler<T> },
): T | null {
  const instance = new SyncWalker<T>(enter, leave)
  return instance.visit(ast, null)
}

/**
 * Walks the AST and applies the provided handlers.
 *
 * @template T - The type of the AST node.
 * @param {T} node - The root node of the AST.
 * @param {WalkHandlers<T, void>} hooks - The handlers to be applied during the walk.
 * @returns {T | null} - The modified AST node or null if the node is removed.
 */
export const walkAST: <T = Node>(
  node: NoInfer<T>,
  hooks: WalkHandlers<T, void>,
) => T | null = walk as any

/**
 * Modified from https://github.com/vuejs/core/blob/main/packages/compiler-core/src/babelUtils.ts
 * To support browser environments and JSX.
 *
 * https://github.com/vuejs/core/blob/main/LICENSE
 */

/**
 * Return value indicates whether the AST walked can be a constant
 */
export function walkIdentifiers(
  root: Node,
  onIdentifier: (
    node: IdentifierName,
    parent: Node | null | undefined,
    parentStack: Node[],
    isReference: boolean,
    isLocal: boolean,
  ) => void,
  includeAll = false,
  parentStack: Node[] = [],
  knownIds: Record<string, number> = Object.create(null),
): void {
  const rootExp =
    root.type === 'Program'
      ? root.body[0].type === 'ExpressionStatement' && root.body[0].expression
      : root

  walkAST<Node>(root, {
    enter(node: Node & { scopeIds?: Set<string> }, parent) {
      parent && parentStack.push(parent)
      if (
        parent &&
        parent.type.startsWith('TS') &&
        !TS_NODE_TYPES.includes(parent.type as any)
      ) {
        return this.skip()
      }
      if (isIdentifier(node)) {
        const isLocal = !!knownIds[node.name]
        const isRefed = isReferencedIdentifier(node, parent, parentStack)
        if (includeAll || (isRefed && !isLocal)) {
          onIdentifier(node, parent, parentStack, isRefed, isLocal)
        }
      } else if (node.type === 'Property' && parent?.type === 'ObjectPattern') {
        // mark property in destructure pattern
        ;(node as any).inPattern = true
      } else if (isFunctionType(node)) {
        /* v8 ignore start */
        if (node.scopeIds) {
          node.scopeIds.forEach((id) => markKnownIds(id, knownIds))
          /* v8 ignore end */
        } else {
          // walk function expressions and add its arguments to known identifiers
          // so that we don't prefix them
          walkFunctionParams(node, (id) =>
            markScopeIdentifier(node, id, knownIds),
          )
        }
      } else if (node.type === 'BlockStatement') {
        /* v8 ignore start */
        if (node.scopeIds) {
          node.scopeIds.forEach((id) => markKnownIds(id, knownIds))
          /* v8 ignore end */
        } else {
          // #3445 record block-level local variables
          walkBlockDeclarations(node, (id) =>
            markScopeIdentifier(node, id, knownIds),
          )
        }
      } else if (node.type === 'CatchClause' && node.param) {
        for (const id of extractIdentifiers(node.param)) {
          markScopeIdentifier(node, id, knownIds)
        }
      } else if (isForStatement(node)) {
        walkForStatement(node, false, (id) =>
          markScopeIdentifier(node, id, knownIds),
        )
      }
    },
    leave(node: Node & { scopeIds?: Set<string> }, parent) {
      parent && parentStack.pop()
      if (node !== rootExp && node.scopeIds) {
        for (const id of node.scopeIds) {
          knownIds[id]--
          if (knownIds[id] === 0) {
            delete knownIds[id]
          }
        }
      }
    },
  })
}

export function walkFunctionParams(
  node: Function,
  onIdent: (id: IdentifierName) => void,
): void {
  for (const p of node.params) {
    for (const id of extractIdentifiers(p)) {
      onIdent(id)
    }
  }
}

export function walkBlockDeclarations(
  block: BlockStatement | Program,
  onIdent: (node: IdentifierName) => void,
): void {
  for (const stmt of block.body) {
    if (stmt.type === 'VariableDeclaration') {
      if (stmt.declare) continue
      for (const decl of stmt.declarations) {
        for (const id of extractIdentifiers(decl.id)) {
          onIdent(id)
        }
      }
    } else if (
      stmt.type === 'FunctionDeclaration' ||
      stmt.type === 'ClassDeclaration'
    ) {
      /* v8 ignore next */
      if (stmt.declare || !stmt.id) continue
      onIdent(stmt.id)
    } else if (isForStatement(stmt)) {
      walkForStatement(stmt, true, onIdent)
    }
  }
}

function walkForStatement(
  stmt: ForStatement | ForOfStatement | ForInStatement,
  isVar: boolean,
  onIdent: (id: IdentifierName) => void,
) {
  const variable = stmt.type === 'ForStatement' ? stmt.init : stmt.left
  if (
    variable &&
    variable.type === 'VariableDeclaration' &&
    (variable.kind === 'var' ? isVar : !isVar)
  ) {
    for (const decl of variable.declarations) {
      for (const id of extractIdentifiers(decl.id)) {
        onIdent(id)
      }
    }
  }
}

function markKnownIds(name: string, knownIds: Record<string, number>) {
  if (name in knownIds) {
    knownIds[name]++
  } else {
    knownIds[name] = 1
  }
}

function markScopeIdentifier(
  node: Node & { scopeIds?: Set<string> },
  child: IdentifierName,
  knownIds: Record<string, number>,
) {
  const { name } = child
  /* v8 ignore start */
  if (node.scopeIds && node.scopeIds.has(name)) {
    return
  }
  /* v8 ignore end */
  markKnownIds(name, knownIds)
  ;(node.scopeIds || (node.scopeIds = new Set())).add(name)
}

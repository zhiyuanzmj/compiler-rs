import { isTemplate } from '@vue-jsx-vapor/compiler-rs'
import { isHTMLTag, isSVGTag } from '@vue/shared'
import { IRNodeTypes, type RootNode, type SimpleExpressionNode } from '../ir'
import { unwrapTSNode } from './utils'
import type {
  Expression,
  ForInStatement,
  ForOfStatement,
  ForStatement,
  Function,
  IdentifierName,
  JSXElement,
  JSXFragment,
  Node,
  ObjectProperty,
} from 'oxc-parser'
export {
  isBigIntLiteral,
  isConstantNode,
  isNumericLiteral,
  isStringLiteral,
  isTemplate,
} from '@vue-jsx-vapor/compiler-rs'

export function isMemberExpression(exp: SimpleExpressionNode) {
  if (!exp.ast) return
  const ret = unwrapTSNode(exp.ast) as Expression
  return (
    ret.type === 'MemberExpression' ||
    (ret.type === 'Identifier' && ret.name !== 'undefined')
  )
}

export const isFragmentNode = (
  node: Node | RootNode,
): node is JSXElement | JSXFragment | RootNode =>
  node.type === IRNodeTypes.ROOT ||
  node.type === 'JSXFragment' ||
  (node.type === 'JSXElement' && !!isTemplate(node))

const nonIdentifierRE = /^$|^\d|[^$\w\u00A0-\uFFFF]/
export const isSimpleIdentifier = (name: string): boolean =>
  !nonIdentifierRE.test(name)

export const isFnExpression: (exp: SimpleExpressionNode) => boolean = (exp) => {
  try {
    if (!exp.ast) return false
    let ret = exp.ast
    // parser may parse the exp as statements when it contains semicolons
    if (ret.type === 'Program') {
      ret = ret.body[0]
      if (ret.type === 'ExpressionStatement') {
        ret = ret.expression
      }
    }
    ret = unwrapTSNode(ret) as Expression
    return (
      ret.type === 'FunctionExpression' ||
      ret.type === 'ArrowFunctionExpression'
    )
  } catch {
    return false
  }
}

export function isJSXComponent(node: Node) {
  if (node.type !== 'JSXElement') return false

  const { openingElement } = node
  if (openingElement.name.type === 'JSXIdentifier') {
    const name = openingElement.name.name
    return !isHTMLTag(name) && !isSVGTag(name)
  } else {
    return openingElement.name.type === 'JSXMemberExpression'
  }
}

/**
 * Checks if the given node is a function type.
 *
 * @param node - The node to check.
 * @returns True if the node is a function type, false otherwise.
 */
export function isFunctionType(
  node: Node | undefined | null,
): node is Function {
  return (
    !!node &&
    !node.type.startsWith('TS') &&
    /Function(?:Expression|Declaration)$|Method$/.test(node.type)
  )
}

/**
 * Checks if the input `node` is a reference to a bound variable.
 *
 * Copied from https://github.com/babel/babel/blob/main/packages/babel-types/src/validators/isReferenced.ts
 *
 * To avoid runtime dependency on `@babel/types` (which includes process references)
 * This file should not change very often in babel but we may need to keep it
 * up-to-date from time to time.
 *
 * @param node - The node to check.
 * @param parent - The parent node of the input `node`.
 * @param grandparent - The grandparent node of the input `node`.
 * @returns True if the input `node` is a reference to a bound variable, false otherwise.
 */
export function isReferenced(
  node: Node,
  parent: Node,
  grandparent?: Node,
): boolean {
  switch (parent.type) {
    // yes: PARENT[NODE]
    // yes: NODE.child
    // no: parent.NODE
    case 'MemberExpression':
      if (parent.property === node) {
        return !!parent.computed
      }
      return parent.object === node

    case 'JSXMemberExpression':
      return parent.object === node
    // no: let NODE = init;
    // yes: let id = NODE;
    case 'VariableDeclarator':
      return parent.init === node

    // yes: () => NODE
    // no: (NODE) => {}
    case 'ArrowFunctionExpression':
      return parent.body === node

    // no: class { #NODE; }
    // no: class { get #NODE() {} }
    // no: class { #NODE() {} }
    // no: class { fn() { return this.#NODE; } }
    case 'PrivateIdentifier':
      return false

    // no: class { NODE() {} }
    // yes: class { [NODE]() {} }
    // no: class { foo(NODE) {} }
    case 'MethodDefinition':
      if (parent.key === node) {
        return !!parent.computed
      }
      return false

    // yes: { [NODE]: "" }
    // no: { NODE: "" }
    // depends: { NODE }
    // depends: { key: NODE }

    // no: class { NODE = value; }
    // yes: class { [NODE] = value; }
    // yes: class { key = NODE; }
    case 'Property':
    case 'AccessorProperty': {
      if (parent.key.type === 'PrivateIdentifier') {
        return parent.key !== node
      }
      if (parent.key === node) {
        return !!parent.computed
      }
      // parent.value === node
      return !grandparent || grandparent.type !== 'ObjectPattern'
    }

    // no: class NODE {}
    // yes: class Foo extends NODE {}
    case 'ClassDeclaration':
    case 'ClassExpression':
      return parent.superClass === node

    // yes: left = NODE;
    // no: NODE = right;
    case 'AssignmentExpression':
      return parent.right === node

    // no: [NODE = foo] = [];
    // yes: [foo = NODE] = [];
    case 'AssignmentPattern':
      return parent.right === node

    // no: NODE: for (;;) {}
    case 'LabeledStatement':
      return false

    // no: try {} catch (NODE) {}
    case 'CatchClause':
      return false

    // no: function foo(...NODE) {}
    case 'RestElement':
      return false

    case 'BreakStatement':
    case 'ContinueStatement':
      return false

    // no: function NODE() {}
    // no: function foo(NODE) {}
    case 'FunctionDeclaration':
    case 'FunctionExpression':
      return false

    // no: export NODE from "foo";
    // no: export * as NODE from "foo";
    case 'ExportAllDeclaration':
      // don't support in oxc
      // case 'ExportDefaultSpecifier':
      return false

    // no: export { foo as NODE };
    // yes: export { NODE as foo };
    // no: export { NODE as foo } from "foo";
    case 'ExportSpecifier':
      if (
        grandparent?.type === 'ExportNamedDeclaration' &&
        grandparent.source
      ) {
        return false
      }
      return parent.local === node

    // no: import NODE from "foo";
    // no: import * as NODE from "foo";
    // no: import { NODE as foo } from "foo";
    // no: import { foo as NODE } from "foo";
    // no: import NODE from "bar";
    case 'ImportDefaultSpecifier':
    case 'ImportNamespaceSpecifier':
    case 'ImportSpecifier':
      return false

    // no: import "foo" assert { NODE: "json" }
    case 'ImportAttribute':
      return false

    // no: <div NODE="foo" />
    // no: <div foo:NODE="foo" />
    case 'JSXAttribute':
    case 'JSXNamespacedName':
      return false

    // no: [NODE] = [];
    // no: ({ NODE }) = [];
    case 'ObjectPattern':
    case 'ArrayPattern':
      return false

    // no: new.NODE
    // no: NODE.target
    case 'MetaProperty':
      return false

    // yes: enum X { Foo = NODE }
    // no: enum X { NODE }
    case 'TSEnumMember':
      return parent.id !== node

    // yes: { [NODE]: value }
    // no: { NODE: value }
    case 'TSPropertySignature':
      if (parent.key === node) {
        return !!parent.computed
      }

      return true
  }

  return true
}

export function isIdentifier(
  node?: Node | undefined | null,
): node is IdentifierName {
  return !!node && (node.type === 'Identifier' || node.type === 'JSXIdentifier')
}

export const isStaticProperty = (node?: Node): node is ObjectProperty =>
  !!node && node.type === 'Property' && !node.computed

export function isForStatement(
  stmt: Node,
): stmt is ForStatement | ForOfStatement | ForInStatement {
  return (
    stmt.type === 'ForOfStatement' ||
    stmt.type === 'ForInStatement' ||
    stmt.type === 'ForStatement'
  )
}

export function isReferencedIdentifier(
  id: IdentifierName,
  parent: Node | null | undefined,
  parentStack: Node[],
): boolean {
  if (!parent) {
    return true
  }

  // is a special keyword but parsed as identifier
  if (id.name === 'arguments') {
    return false
  }

  if (isReferenced(id, parent, parentStack.at(-2))) {
    return true
  }

  // babel's isReferenced check returns false for ids being assigned to, so we
  // need to cover those cases here
  switch (parent.type) {
    case 'AssignmentExpression':
    case 'AssignmentPattern':
      return true
    case 'Property':
      return parent.key !== id && isInDestructureAssignment(parent, parentStack)
    case 'ArrayPattern':
      return isInDestructureAssignment(parent, parentStack)
  }

  return false
}

export function isInDestructureAssignment(
  parent: Node,
  parentStack: Node[],
): boolean {
  if (
    parent &&
    (parent.type === 'Property' || parent.type === 'ArrayPattern')
  ) {
    let i = parentStack.length
    while (i--) {
      const p = parentStack[i]
      if (p.type === 'AssignmentExpression') {
        return true
      } else if (p.type !== 'Property' && !p.type.endsWith('Pattern')) {
        break
      }
    }
  }
  return false
}

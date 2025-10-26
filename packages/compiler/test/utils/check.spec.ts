// @ts-nocheck
import {
  isReferenced,
  isReferencedIdentifier,
  walkIdentifiers,
} from '@vue-jsx-vapor/compiler-rs'
import { parseSync } from 'oxc-parser'
import { describe, expect, test, vi } from 'vitest'

describe('isReferenced', () => {
  test('member', () => {
    const node = parseSync('index.ts', 'foo.bar').program.body[0].expression
    expect(isReferenced(node.object, node)).toBe(true)
    expect(isReferenced(node.property, node)).toBe(false)

    const node2 = parseSync('index.ts', 'foo[bar]').program.body[0].expression
    expect(isReferenced(node2.property, node2)).toBe(true)
  })

  test('class', () => {
    const node = parseSync('index.ts', 'class Foo {}').program.body[0]
    expect(isReferenced(node.id!, node)).toBe(false)

    const node2 = parseSync('index.ts', 'class extends Foo {}').program.body[0]
    expect(isReferenced(node2.superClass!, node2))
  })
})

describe('isReferencedIdentifier', () => {
  test('identifier is referenced in a variable declaration', () => {
    expect.assertions(1)
    const ast = parseSync('index.ts', `const a = b`).program
    walkIdentifiers(ast.body[0], (node, parent, parentStack) => {
      if (node.type === 'Identifier' && node.name === 'b') {
        expect(isReferencedIdentifier(node, parent, parentStack)).toBe(true)
      }
    })
  })

  test('identifier is referenced in a function call', () => {
    expect.assertions(1)
    const ast = parseSync('index.ts', `foo(bar)`).program
    walkIdentifiers(ast.body[0], (node, parent, parentStack) => {
      if (node.type === 'Identifier' && node.name === 'bar') {
        expect(isReferencedIdentifier(node, parent, parentStack)).toBe(true)
      }
    })
  })

  test('identifier is referenced in a member expression', () => {
    expect.assertions(1)
    const ast = parseSync('index.ts', `obj.prop`).program
    walkIdentifiers(ast.body[0], (node, parent, parentStack) => {
      if (node.type === 'Identifier' && node.name === 'obj') {
        expect(isReferencedIdentifier(node, parent, parentStack)).toBe(true)
      }
    })
  })

  test('identifier is referenced in an assignment expression', () => {
    expect.assertions(1)
    const ast = parseSync('index.ts', `a = b`).program
    walkIdentifiers(ast.body[0], (node, parent, parentStack) => {
      if (node.type === 'Identifier' && node.name === 'b') {
        expect(isReferencedIdentifier(node, parent, parentStack)).toBe(true)
      }
    })
  })

  test('identifier is referenced in a return statement', () => {
    expect.assertions(1)
    const ast = parseSync('index.ts', `function foo() { return bar }`).program
    walkIdentifiers(ast.body[0], (node, parent, parentStack) => {
      if (node.type === 'Identifier' && node.name === 'bar') {
        expect(isReferencedIdentifier(node, parent, parentStack)).toBe(true)
      }
    })
  })

  test('identifier is referenced in a conditional expression', () => {
    expect.assertions(2)
    const ast = parseSync('index.ts', `a ? b : c`).program
    walkIdentifiers(ast.body[0], (node, parent, parentStack) => {
      if (
        node.type === 'Identifier' &&
        (node.name === 'b' || node.name === 'c')
      ) {
        expect(isReferencedIdentifier(node, parent, parentStack)).toBe(true)
      }
    })
  })

  test('identifiers in function parameters should not be inferred as references', () => {
    expect.assertions(4)
    const ast = parseSync('index.ts', `(({ title }) => [])`).program
    walkIdentifiers(
      ast.body[0],
      (node, parent, parentStack, isReference) => {
        expect(isReference).toBe(false)
        expect(isReferencedIdentifier(node, parent, parentStack)).toBe(false)
      },
      true,
    )
  })

  test('JSXNamespacedName should not be inferred as references', () => {
    const ast = parseSync(
      'index.tsx',
      `const Comp = <svg:circle foo:bar="" />`,
    ).program
    const onIdentifier = vi.fn()
    walkIdentifiers(ast.body[0], onIdentifier)
    expect(onIdentifier).toHaveBeenCalledTimes(0)
  })
})

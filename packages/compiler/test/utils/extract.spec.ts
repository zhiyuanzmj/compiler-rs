// @ts-nocheck
import { extractIdentifiers } from '@vue-jsx-vapor/compiler-rs'
import { parseSync, type IdentifierName } from 'oxc-parser'
import { describe, expect, test } from 'vitest'

describe('extract', () => {
  test('extractIdentifiers', () => {
    const ast = parseSync(
      'index.ts',
      `
      const one = 1
      const {
        propA,
        propB: aliasPropB,
        propC = 33,
        propD: aliasPropD = 44,
        ...objRest
      } = {
        propA: 1,
        propB: 2,
        propC: 3,
        propD: 4,
        propE: 5,
        propF: 6,
      }
      const [elOne, elTwo = 22, ...elRest] = [1, 2, 3, 4]
      memberExpressionObj.a
    `,
    ).program

    let identifiers: IdentifierName[] = []

    for (const b of ast.body) {
      if (b.type === 'VariableDeclaration') {
        for (const d of b.declarations) {
          identifiers = extractIdentifiers(d.id, identifiers)
        }
      }

      if (b.type === 'ExpressionStatement') {
        identifiers = extractIdentifiers(b.expression, identifiers)
      }
    }

    expect(
      identifiers.map((id) => ({
        name: id.name,
      })),
    ).toMatchInlineSnapshot(`
      [
        {
          "name": "one",
        },
        {
          "name": "propA",
        },
        {
          "name": "aliasPropB",
        },
        {
          "name": "propC",
        },
        {
          "name": "aliasPropD",
        },
        {
          "name": "objRest",
        },
        {
          "name": "elOne",
        },
        {
          "name": "elTwo",
        },
        {
          "name": "elRest",
        },
        {
          "name": "memberExpressionObj",
        },
      ]
    `)
  })
})

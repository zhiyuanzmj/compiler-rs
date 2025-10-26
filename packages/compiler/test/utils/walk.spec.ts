// @ts-nocheck
import { walk, walkIdentifiers } from '@vue-jsx-vapor/compiler-rs'
import { parseSync } from 'oxc-parser'
import { assert, describe, expect, it, test } from 'vitest'

describe('sync estree-walker', () => {
  it('walks a malformed node', () => {
    const block = [
      {
        type: 'Foo',
        answer: undefined,
      },
      {
        type: 'Foo',
        answer: {
          type: 'Answer',
          value: 42,
        },
      },
    ]

    let answer

    walk(
      { type: 'Test', block },
      {
        enter(node) {
          if (node.type === 'Answer') answer = node
        },
      },
    )

    assert.equal(answer, block[1].answer)
  })

  it('walks an AST', () => {
    const ast = {
      type: 'Program',
      body: [
        {
          type: 'VariableDeclaration',
          declarations: [
            {
              type: 'VariableDeclarator',
              id: { type: 'Identifier', name: 'a' },
              init: { type: 'Literal', value: 1, raw: '1' },
            },
            {
              type: 'VariableDeclarator',
              id: { type: 'Identifier', name: 'b' },
              init: { type: 'Literal', value: 2, raw: '2' },
            },
          ],
          kind: 'var',
        },
      ],
      sourceType: 'module',
    }

    const entered = []
    const left = []

    walk(ast, {
      enter(node) {
        entered.push(node)
      },
      leave(node) {
        left.push(node)
      },
    })

    expect(entered).toMatchObject([
      ast,
      ast.body[0],
      ast.body[0].declarations[0],
      ast.body[0].declarations[0].id,
      ast.body[0].declarations[0].init,
      ast.body[0].declarations[1],
      ast.body[0].declarations[1].id,
      ast.body[0].declarations[1].init,
    ])

    expect(left).toMatchObject([
      ast.body[0].declarations[0].id,
      ast.body[0].declarations[0].init,
      ast.body[0].declarations[0],
      ast.body[0].declarations[1].id,
      ast.body[0].declarations[1].init,
      ast.body[0].declarations[1],
      ast.body[0],
      ast,
    ])
  })

  it('handles null literals', () => {
    const ast = {
      type: 'Program',
      start: 0,
      end: 8,
      body: [
        {
          type: 'ExpressionStatement',
          start: 0,
          end: 5,
          expression: {
            type: 'Literal',
            start: 0,
            end: 4,
            value: null,
            raw: 'null',
          },
        },
        {
          type: 'ExpressionStatement',
          start: 6,
          end: 8,
          expression: {
            type: 'Literal',
            start: 6,
            end: 7,
            value: 1,
            raw: '1',
          },
        },
      ],
      sourceType: 'module',
    }

    walk(ast, {
      enter() {},
      leave() {},
    })

    assert.ok(true)
  })

  it('allows walk() to be invoked within a walk, without context corruption', () => {
    const ast = {
      type: 'Program',
      start: 0,
      end: 8,
      body: [
        {
          type: 'ExpressionStatement',
          start: 0,
          end: 6,
          expression: {
            type: 'BinaryExpression',
            start: 0,
            end: 5,
            left: {
              type: 'Identifier',
              start: 0,
              end: 1,
              name: 'a',
            },
            operator: '+',
            right: {
              type: 'Identifier',
              start: 4,
              end: 5,
              name: 'b',
            },
          },
        },
      ],
      sourceType: 'module',
    }

    const identifiers = []

    walk(ast, {
      enter(node) {
        if (node.type === 'ExpressionStatement') {
          walk(node, function enter() {
            return true
          })
        }

        if (node.type === 'Identifier') {
          identifiers.push(node.name)
        }
      },
    })

    expect(identifiers).toMatchObject(['a', 'b'])
  })

  it('replaces a node', () => {
    const phases = ['enter', 'leave']
    for (const phase of phases) {
      const ast = {
        type: 'Program',
        start: 0,
        end: 8,
        body: [
          {
            type: 'ExpressionStatement',
            start: 0,
            end: 6,
            expression: {
              type: 'BinaryExpression',
              start: 0,
              end: 5,
              left: {
                type: 'Identifier',
                start: 0,
                end: 1,
                name: 'a',
              },
              operator: '+',
              right: {
                type: 'Identifier',
                start: 4,
                end: 5,
                name: 'b',
              },
            },
          },
        ],
        sourceType: 'module',
      }

      const forty_two = {
        type: 'Literal',
        value: 42,
        raw: '42',
      }

      walk(ast, {
        [phase](node) {
          if (node.type === 'Identifier' && node.name === 'b') {
            return forty_two
          }
        },
      })

      assert.equal(ast.body[0].expression.right, forty_two)
    }
  })

  it('replaces a top-level node', () => {
    const ast = {
      type: 'Identifier',
      name: 'answer',
    }

    const forty_two = {
      type: 'Literal',
      value: 42,
      raw: '42',
    }

    const node = walk(ast, {
      enter(node) {
        if (node.type === 'Identifier' && node.name === 'answer') {
          return forty_two
        }
      },
    })

    expect(node).toMatchObject(forty_two)
  })

  it('removes a node property', () => {
    const phases = ['enter', 'leave']
    for (const phase of phases) {
      const ast = {
        type: 'Program',
        start: 0,
        end: 8,
        body: [
          {
            type: 'ExpressionStatement',
            start: 0,
            end: 6,
            expression: {
              type: 'BinaryExpression',
              start: 0,
              end: 5,
              left: {
                type: 'Identifier',
                start: 0,
                end: 1,
                name: 'a',
              },
              operator: '+',
              right: {
                type: 'Identifier',
                start: 4,
                end: 5,
                name: 'b',
              },
            },
          },
        ],
        sourceType: 'module',
      }

      walk(ast, {
        [phase](node) {
          if (node.type === 'Identifier' && node.name === 'b') {
            return false
          }
        },
      })

      assert.equal(ast.body[0].expression.right, undefined)
    }
  })

  it('removes a node from array', () => {
    const phases = ['enter', 'leave']
    for (const phase of phases) {
      const ast = {
        type: 'Program',
        body: [
          {
            type: 'VariableDeclaration',
            declarations: [
              {
                type: 'VariableDeclarator',
                id: {
                  type: 'Identifier',
                  name: 'a',
                },
                init: null,
              },
              {
                type: 'VariableDeclarator',
                id: {
                  type: 'Identifier',
                  name: 'b',
                },
                init: null,
              },
              {
                type: 'VariableDeclarator',
                id: {
                  type: 'Identifier',
                  name: 'c',
                },
                init: null,
              },
            ],
            kind: 'let',
          },
        ],
        sourceType: 'module',
      }

      const visitedIndex = []

      walk(ast, {
        [phase](node, parent, key, index) {
          if (node.type === 'VariableDeclarator') {
            visitedIndex.push(index)
            if (node.id.name === 'a' || node.id.name === 'b') {
              return false
            }
          }
        },
      })

      assert.equal(ast.body[0].declarations.length, 1)
      assert.equal(visitedIndex.length, 3)
      expect(visitedIndex).toMatchObject([0, 0, 0])
      assert.equal(ast.body[0].declarations[0].id.name, 'c')
    }
  })
})

describe('walkIdentifiers', () => {
  test('JSXIdentifier', () => {
    const ast = parseSync(
      'index.tsx',
      `
      function Comp({ Foo }){
      const a = 1
        return <Foo />
      }
      `,
    ).program.body[0]
    walkIdentifiers(ast, (id) => {
      expect(id.name).toBe('Foo')
    })
  })

  test('JSXMemberExpression', () => {
    const ast = parseSync(
      'index.tsx',
      `
      function Comp(props){
        return <props.Foo />
      }
      `,
    )

    walkIdentifiers(ast.program.body[0].body, (id) => {
      expect(id.name).toBe('props')
    })
  })

  test('nested identifiers', () => {
    const ast = parseSync(
      'index.ts',
      `
    function nested() {
      const a = 1;
      function inner() {
      const b = 2;
      return a + b;
      }
      return inner();
    }
    `,
    ).program
    const identifiers: string[] = []
    walkIdentifiers(
      ast.body[0].body,
      (id) => {
        identifiers.push(id.name)
      },
      true,
    )
    expect(identifiers).toEqual(['a', 'inner', 'b', 'a', 'b', 'inner'])
  })

  test('object pattern destructuring', () => {
    const ast = parseSync(
      'index.ts',
      `
    function destructure({ x, y }) {
      return x + y;
    }
    `,
    ).program
    const identifiers: string[] = []
    walkIdentifiers(ast.body[0].body, (id) => {
      identifiers.push(id.name)
    })
    expect(identifiers).toEqual(['x', 'y'])
  })

  test('array pattern destructuring', () => {
    const ast = parseSync(
      'index.ts',
      `
    function destructure([a, b]) {
      return a + b;
    }
    `,
    ).program
    const identifiers: string[] = []
    walkIdentifiers(ast.body[0].body, (id) => {
      identifiers.push(id.name)
    })
    expect(identifiers).toEqual(['a', 'b'])
  })

  test('catch clause identifiers', () => {
    const ast = parseSync(
      'ts',
      `
    try {
      throw new Error('test');
    } catch (error) {
      ;[error]
    }
    `,
    ).program
    const identifiers: string[] = []
    walkIdentifiers(
      (ast.body[0] as any).handler,
      (id) => {
        identifiers.push(id.name)
      },
      true,
    )
    expect(identifiers).toEqual(['error', 'error'])
  })

  test('for loop identifiers', () => {
    const ast = parseSync(
      'index.ts',
      `
    for (let i = 0; i < 10; i++) {
      ;[i]
    }
    `,
    ).program
    const identifiers: string[] = []
    walkIdentifiers(
      ast,
      (id, _, __, isReference) => {
        if (isReference) identifiers.push(id.name)
      },
      true,
    )
    expect(identifiers).toEqual(['i', 'i', 'i'])
  })

  test('function parameters', () => {
    const ast = parseSync(
      'index.ts',
      `
    function params(a, b, c) {
      return a + b + c;
      function inner(a, b, c) {
        return a + b + c;
      }
    }
    `,
    ).program
    const identifiers: string[] = []
    walkIdentifiers(
      ast,
      (id) => {
        identifiers.push(id.name)
      },
      true,
    )
    expect(identifiers).toEqual([
      'params',
      'a',
      'b',
      'c',
      'a',
      'b',
      'c',
      'inner',
      'a',
      'b',
      'c',
      'a',
      'b',
      'c',
    ])
  })

  test('ignore type annotations', () => {
    const ast = parseSync(
      'index.ts',
      `
      function typed(a: number, b: string): void {
        ;[a, b]
      }`,
    ).program
    const identifiers: string[] = []
    walkIdentifiers(
      ast,
      (id) => {
        identifiers.push(id.name)
      },
      true,
    )
    expect(identifiers).toEqual(['typed', 'a', 'b', 'a', 'b'])
  })

  test('walk ExpressionStatement', () => {
    const ast = parseSync('index.ts', 'a').program
    walkIdentifiers(ast, (id) => {
      expect(id.name).toBe('a')
    })
  })
})

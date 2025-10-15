// TODO: add tests for this transform
import { describe, expect, it, test } from 'vitest'
import {
  IRNodeTypes,
  transformChildren,
  transformElement,
  transformText,
  transformVBind,
  transformVOn,
  transformVOnce,
} from '../../src'

import { makeCompile } from './_utils'

const compileWithTextTransform = makeCompile({
  nodeTransforms: [
    transformVOnce,
    transformElement,
    transformText,
    transformChildren,
  ],
  directiveTransforms: {
    bind: transformVBind,
    on: transformVOn,
  },
})

describe('compiler: text transform', () => {
  it('no consecutive text', () => {
    const { code, ir, helpers } = compileWithTextTransform(
      '<>{ "hello world" }</>',
    )
    expect(code).toMatchSnapshot()
    expect(helpers).contains.all.keys('createNodes')
    expect(ir.block.operation).toMatchObject([
      {
        type: IRNodeTypes.CREATE_NODES,
        values: [
          {
            content: 'hello world',
            isStatic: true,
          },
        ],
      },
    ])
  })

  it('consecutive text', () => {
    const { code, ir, helpers } = compileWithTextTransform('<>{ msg }</>')
    expect(code).toMatchSnapshot()
    expect(helpers).contains.all.keys('createNodes')
    expect(ir.block.operation).toMatchObject([
      {
        type: IRNodeTypes.CREATE_NODES,
        values: [
          {
            content: 'msg',
            isStatic: false,
          },
        ],
      },
    ])
    expect(ir.block.effect.length).toBe(0)
  })

  it('escapes raw static text when generating the template string', () => {
    const { ir } = compileWithTextTransform('<code>&lt;script&gt;</code>')
    expect(ir.templates[0]).toContain('<code>&lt;script&gt;</code>')
    expect(ir.templates[0]).not.toContain('<code><script></code>')
  })

  it('text like', () => {
    const { ir, code } = compileWithTextTransform('<div>{`foo`}{1}{1n}</div>')
    expect(code).toMatchSnapshot()
    expect(ir.templates[0]).not.toContain('setNodes')
  })
})

describe('compiler: expression', () => {
  test('conditional expression', () => {
    const { code, helpers, ir } = compileWithTextTransform(
      `<>{ok? <span>{msg}</span> : fail ? <div>fail</div>  : null }</>`,
    )

    expect(code).toMatchSnapshot()

    expect(helpers).contains('createIf')

    expect(ir.templates).toEqual(['<span> </span>', '<div>fail</div>'])
    const op = ir.block.dynamic.children[0].operation
    expect(op).toMatchObject({
      type: IRNodeTypes.IF,
      id: 0,
      condition: {
        content: 'ok',
        isStatic: false,
      },
      positive: {
        type: IRNodeTypes.BLOCK,
        dynamic: {
          children: [{ template: 0 }],
        },
      },
    })
    expect(ir.block.returns).toEqual([0])
    expect(ir.block.dynamic).toMatchObject({
      children: [{ id: 0 }],
    })

    expect(ir.block.effect).toEqual([])
    expect(op.positive.effect).lengthOf(0)

    expect(code).matchSnapshot()
  })
  test('logical expression', () => {
    const { code, helpers, ir } = compileWithTextTransform(
      `<>{ok && <div>{msg}</div>}</>`,
    )

    expect(helpers).contains('createIf')

    expect(ir.templates).toEqual(['<div> </div>'])
    const op = ir.block.dynamic.children[0].operation
    expect(op).toMatchObject({
      type: IRNodeTypes.IF,
      id: 0,
      condition: {
        content: 'ok',
        isStatic: false,
      },
      positive: {
        type: IRNodeTypes.BLOCK,
        dynamic: {
          children: [{ template: 0 }],
        },
      },
    })
    expect(ir.block.returns).toEqual([0])
    expect(ir.block.dynamic).toMatchObject({
      children: [{ id: 0 }],
    })

    expect(ir.block.effect).toEqual([])
    expect(op.positive.effect).lengthOf(0)
    expect(code).toMatchSnapshot()
  })
  test('conditional expression with v-once', () => {
    const { code, helpers, ir } = compileWithTextTransform(
      `<div v-once>{ok? <span>{msg}</span> : <div>fail</div> }</div>`,
    )
    expect(code).toMatchSnapshot()

    expect(helpers).contains('createIf')
    expect(ir.templates).toEqual([
      '<span> </span>',
      '<div>fail</div>',
      '<div></div>',
    ])
    expect(ir.block.returns).toEqual([5])
    expect(ir.block.dynamic).toMatchObject({
      children: [{ id: 5 }],
    })
  })
})

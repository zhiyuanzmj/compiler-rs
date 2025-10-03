// @ts-nocheck
import { describe, expect, test } from 'vitest'
import {
  getTextLikeValue,
  isBigIntLiteral,
  isNumericLiteral,
  isStringLiteral,
  isTemplate,
  unwrapTSNode,
} from '../index.js'

describe('utils', () => {
  test('unwrapTSNode', () => {
    expect(unwrapTSNode({ type: 'TSAsExpression', expression: {} })).toMatchInlineSnapshot(`
      {}
    `)
  })
  test('isStringLiteral', () => {
    expect(isStringLiteral({ type: 'Literal', value: 'hello' })).toBe(true)
    expect(isStringLiteral({ type: 'Literal', value: 123 })).toBe(false)
    expect(isStringLiteral({})).toBe(false)
  })

  test('isNumericeLiteral', () => {
    expect(isNumericLiteral({ type: 'Literal', value: 'hello' })).toBe(false)
    expect(isNumericLiteral({ type: 'Literal', value: 123 })).toBe(true)
    expect(isNumericLiteral({ type: 'Literal', value: -123 })).toBe(true)
    expect(isNumericLiteral({ type: 'Literal', value: 1.1 })).toBe(true)
    expect(isNumericLiteral({})).toBe(false)
  })

  test('isBigIntLiteral', () => {
    expect(isBigIntLiteral({ type: 'Literal', value: 1n })).toBe(true)
  })

  test('isTemplate', () => {
    expect(
      isTemplate({ type: 'JSXElement', openingElement: { name: { type: 'JSXIdentifier', name: 'template' } } }),
    ).toBe(true)
    expect(isTemplate({ type: 'JSXElement', openingElement: {} })).toBe(false)
  })

  test('getTextLikeValue', () => {
    // expect(getTextLikeValue({ type: 'Literal', value: 'foo' })).toBe('foo')
    expect(getTextLikeValue({ type: 'Literal', value: 1 })).toBe('1')
    // expect(getTextLikeValue({ type: 'Literal', value: 1n })).toBe('1')
    // expect(getTextLikeValue({ type: 'TemplateLiteral', expressions: [], quasis: [{ value: { cooked: 'foo' } }] })).toBe(
    //   'foo',
    // )
  })
})

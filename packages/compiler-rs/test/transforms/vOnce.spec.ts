import { compile } from '@vue-jsx-vapor/compiler-rs'
import { describe, expect, test } from 'vitest'

describe('compiler: v-once', () => {
  test('basic', () => {
    const { code } = compile(
      `<div v-once>
        { msg }
        <span class={clz} />
      </div>`,
    )

    expect(code).toMatchSnapshot()
  })

  test('as root node', () => {
    const { code } = compile(`<div id={foo} v-once />`)

    expect(code).toMatchSnapshot()
    expect(code).not.contains('effect')
  })

  test('on nested plain element', () => {
    const { code } = compile(`<div><div id={foo} v-once /></div>`)

    expect(code).toMatchSnapshot()
  })

  test('on component', () => {
    const { code } = compile(`<div><Comp id={foo} v-once /></div>`)
    expect(code).toMatchSnapshot()
  })

  test.todo('on slot outlet')

  test('inside v-once', () => {
    const { code } = compile(`<div v-once><div v-once/></div>`)

    expect(code).toMatchSnapshot()
  })

  test('with v-if', () => {
    const { code } = compile(`<div v-if={expr} v-once />`)
    expect(code).toMatchSnapshot()
  })

  test('with v-if/else', () => {
    const { code } = compile(`<><div v-if={expr} v-once /><p v-else/></>`)
    expect(code).toMatchSnapshot()
  })

  test('with v-for', () => {
    const { code } = compile(`<div v-for={i in list} v-once />`)
    expect(code).toMatchSnapshot()
  })
})

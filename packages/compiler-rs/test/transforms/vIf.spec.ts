import { compile } from '@vue-jsx-vapor/compiler-rs'
import { describe, expect, test } from 'vitest'

describe('compiler: v-if', () => {
  test('basic v-if', () => {
    const { code, helpers } = compile(`<div v-if={ok}>{msg}</div>`)

    expect(helpers).contains('createIf')
    expect(code).toMatchSnapshot()
  })

  test('template v-if', () => {
    const { code, templates } = compile(`<template v-if={ok}><div/>hello<p v-text={msg}></p></template>`)
    expect(code).matchSnapshot()
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<div></div>",
          false,
        ],
        [
          "hello",
          false,
        ],
        [
          "<p> </p>",
          false,
        ],
      ]
    `)
  })

  test('dedupe same template', () => {
    const { code, templates } = compile(`<><div v-if={ok}>hello</div><div v-if={ok}>hello</div></>`)
    expect(code).matchSnapshot()
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<div>hello</div>",
          false,
        ],
      ]
    `)
  })

  // test.todo('component v-if')

  test('v-if + v-else', () => {
    const { code, helpers, templates } = compile(`<><div v-if={ok}/><p v-else/></>`)
    expect(code).matchSnapshot()
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<div></div>",
          false,
        ],
        [
          "<p></p>",
          false,
        ],
      ]
    `)

    expect(helpers).contains('createIf')
  })

  test('v-if + v-else-if', () => {
    const { code, templates } = compile(`<><div v-if={ok}/><p v-else-if={orNot}/></>`)
    expect(code).matchSnapshot()
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<div></div>",
          false,
        ],
        [
          "<p></p>",
          false,
        ],
      ]
    `)
  })

  test('v-if + v-else-if + v-else', () => {
    const { code, templates } = compile(`<><div v-if={ok}/><p v-else-if={orNot}/><template v-else>fine</template></>`)
    expect(code).matchSnapshot()
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<div></div>",
          false,
        ],
        [
          "<p></p>",
          false,
        ],
        [
          "fine",
          false,
        ],
      ]
    `)
  })

  test('v-if + v-if / v-else[-if]', () => {
    const { code } = compile(
      `<div>
        <span v-if="foo">foo</span>
        <span v-if="bar">bar</span>
        <span v-else>baz</span>
      </div>`,
    )
    expect(code).toMatchSnapshot()
  })

  test('comment between branches', () => {
    const { code, templates } = compile(
      `
      <>
        <div v-if={ok}/>
        {/* foo */}
        <p v-else-if={orNot}/>
        {/* bar */}
        <template v-else>fine{/* fine */}</template>
        <div v-text="text" />
      </>
    `,
    )
    expect(code).toMatchSnapshot()
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<div></div>",
          false,
        ],
        [
          "<p></p>",
          false,
        ],
        [
          "fine",
          false,
        ],
        [
          "<div>text</div>",
          false,
        ],
      ]
    `)
  })

  describe.todo('errors')
  describe.todo('codegen')
  test.todo('v-on with v-if')
})

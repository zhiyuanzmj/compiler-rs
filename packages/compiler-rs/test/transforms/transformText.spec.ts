import { compile } from '@vue-jsx-vapor/compiler-rs'
import { describe, expect, it, test } from 'vitest'

describe('compiler: text transform', () => {
  it('no consecutive text', () => {
    const { code, helpers } = compile('<>{ "hello world" }</>')
    expect(code).toMatchSnapshot()
    expect(helpers).contains.all.keys('createNodes')
  })

  it('consecutive text', () => {
    const { code, helpers } = compile('<>{ msg }</>')
    expect(code).toMatchSnapshot()
    expect(helpers).contains.all.keys('createNodes')
  })

  it('escapes raw static text when generating the template string', () => {
    const { templates } = compile('<code>&lt;script&gt;</code>')
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<code>&lt;script&gt;</code>",
          true,
        ],
      ]
    `)
  })

  it('text like', () => {
    const { code, templates } = compile('<div>{ (2) }{`foo${1}`}{1}{1n}</div>')
    expect(code).toMatchSnapshot()
    expect(templates[0]).toMatchInlineSnapshot(`
      [
        "<div>2foo111</div>",
        true,
      ]
    `)
  })
})

describe('compiler: expression', () => {
  test('conditional expression', () => {
    const { code, helpers, templates } = compile(`<>{ok? (<span>{msg}</span>) : fail ? (<div>fail</div>)  : null }</>`)
    expect(code).toMatchSnapshot()
    expect(helpers).contains('createIf')
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<span> </span>",
          false,
        ],
        [
          "<div>fail</div>",
          false,
        ],
      ]
    `)
  })
  test('logical expression', () => {
    const { code, helpers, templates } = compile(`<>{ok && (<div>{msg}</div>)}</>`)

    expect(helpers).contains('createIf')
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<div> </div>",
          false,
        ],
      ]
    `)
    expect(code).toMatchSnapshot()
  })
  test('conditional expression with v-once', () => {
    const { code, helpers, templates } = compile(`<div v-once>{ok? <span>{msg}</span> : <div>fail</div> }</div>`)
    expect(code).toMatchSnapshot()

    expect(helpers).contains('createIf')
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<span> </span>",
          false,
        ],
        [
          "<div>fail</div>",
          false,
        ],
        [
          "<div></div>",
          true,
        ],
      ]
    `)
  })

  test('map expression', () => {
    const { code } = compile(`<>{Array.from({ length: count.value }).map((_, index) => {
        if (index > 1) {
          return <div>1</div>
        } else {
          return [<span>({index}) lt 1</span>, <br />]
        }
      })}</>`)
    expect(code).toMatchSnapshot()
  })
})

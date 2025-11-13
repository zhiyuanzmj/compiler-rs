import { compile } from '@vue-jsx-vapor/compiler-rs'
import { describe, expect, test } from 'vitest'

describe('compiler: v-for', () => {
  test('basic v-for', () => {
    const { code, templates, helpers } = compile(
      `<div v-for={item in items} key={item.id} onClick={() => remove(item)}>{item}</div>`,
    )

    expect(code).toMatchSnapshot()

    expect(helpers).contains('createFor')
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<div> </div>",
          false,
        ],
      ]
    `)
  })

  test('key only binding pattern', () => {
    expect(
      compile(
        `
          <tr
            v-for={row in rows}
            key={row.id}
          >
            { row.id + row.id }
          </tr>
      `,
      ).code,
    ).matchSnapshot()
  })

  test('selector pattern', () => {
    expect(
      compile(
        `
          <tr
            v-for={row in rows}
            key={row.id}
            v-text={selected === row.id ? 'danger' : ''}
          ></tr>
      `,
      ).code,
    ).matchSnapshot()
    expect(
      compile(
        `
          <tr
            v-for={row in rows}
            key={row.id}
            class={selected === row.id ? 'danger' : ''}
          ></tr>
      `,
      ).code,
    ).matchSnapshot()
    // Should not be optimized because row.label is not from parent scope
    expect(
      compile(
        `
          <tr
            v-for={row in rows}
            key={row.id}
            class={row.label === row.id ? 'danger' : ''}
          ></tr>
      `,
      ).code,
    ).matchSnapshot()
    expect(
      compile(
        `
          <tr
            v-for={row in rows}
            key={row.id}
            class={{ danger: row.id === selected }}
          ></tr>
      `,
      ).code,
    ).matchSnapshot()
  })

  test('multi effect', () => {
    const { code } = compile(`<div v-for={(item, index) in items} item={item} index={index} />`)
    expect(code).matchSnapshot()
  })

  test('nested v-for', () => {
    const { code, templates } = compile(`<div v-for={i in list}><span v-for={j in i}>{ j+i }</span></div>`)
    expect(code).matchSnapshot()
    expect(code).contains(`_createFor(() => (list), (_for_item0) => {`)
    expect(code).contains(`_createFor(() => (_for_item0.value), (_for_item1) => {`)
    expect(code).contains(`_for_item1.value+_for_item0.value`)
    expect(templates).toMatchInlineSnapshot(`
      [
        [
          "<span> </span>",
          false,
        ],
        [
          "<div></div>",
          false,
        ],
      ]
    `)
  })

  test('object value, key and index', () => {
    const { code } = compile('<span v-for={(value, key, index) in items} key={id}>{ id }{ value }{ index }</span>')
    expect(code).matchSnapshot()
  })

  test('object de-structured value', () => {
    const { code } = compile('<span v-for={({ id, value }) in items} key={id}>{ id }{ value }</span>')
    expect(code).matchSnapshot()
  })

  test('object de-structured value (with rest)', () => {
    const { code } = compile(`<div v-for={(  { id, ...other }, index) in list} key={id}>{ id + other + index }</div>`)
    expect(code).matchSnapshot()
    expect(code).toContain('_getRestElement(_for_item0.value, ["id"])')
  })

  test('array de-structured value', () => {
    const { code } = compile(`<div v-for={([id, other], index) in list} key={id}>{ id + other + index }</div>`)
    expect(code).matchSnapshot()
  })

  test('array de-structured value (with rest)', () => {
    const { code } = compile(`<div v-for={([id, ...other], index) in list} key={id}>{ id + other + index }</div>`)
    expect(code).matchSnapshot()
    expect(code).toContain('_for_item0.value.slice(1)')
  })

  test('v-for aliases w/ complex expressions', () => {
    const { code } = compile(
      `<div v-for={({ foo, baz: [qux] }) in list}>
        { foo + baz + qux }
      </div>`,
    )
    expect(code).matchSnapshot()
  })
  test('fast-remove flag', () => {
    const { code } = compile(
      `<div>
        <span v-for={j in i}>{ j+i }</span>
      </div>`,
    )
    expect(code).toMatchSnapshot()
  })

  test('v-for on component', () => {
    const { code } = compile(`<Comp v-for={item in list}>{item}</Comp>`)
    expect(code).matchSnapshot()
  })

  test('v-for on template with single component child', () => {
    const { code } = compile(`<template v-for={item in list}><Comp>{item}</Comp></template>`)
    expect(code).matchSnapshot()
  })

  test('v-for identifiers', () => {
    const { code } = compile(
      `<div v-for={(item, index) in items} id={index}>
        { ((item) => {
          let index = 1
          return [item, index]
        })(item) }
        { (() => {
          switch (item) {
            case index: {
              let item = ''
              return \`\${[item, index]}\`;
            }
          }
        })() }
      </div>`,
    )

    expect(code).toMatchSnapshot()
  })
})

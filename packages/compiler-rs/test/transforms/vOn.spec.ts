import { compile, ErrorCodes } from '@vue-jsx-vapor/compiler-rs'
import { describe, expect, test, vi } from 'vitest'

describe('v-on', () => {
  test('simple expression', () => {
    const { code, helpers } = compile(`<div onClick={handleClick}></div>`)

    expect(code).toMatchInlineSnapshot(`
      "
        const n0 = t0()
        n0.$evtclick = handleClick
        return n0
      "
    `)
    expect(helpers).not.contains('delegate') // optimized as direct attachment
  })

  test('event modifier', () => {
    const { code } = compile(
      `<>
        <a onClick_stop={handleEvent}></a>
        <form onSubmit_prevent={handleEvent}></form>
        <a onClick_stop_prevent={handleEvent}></a>
        <div onClick_self={handleEvent}></div>
        <div onClick_capture={handleEvent}></div>
        <a onClick_once={handleEvent}></a>
        <div onScroll_passive={handleEvent}></div>
        <input onClick_right={handleEvent} />
        <input onClick_left={handleEvent} />
        <input onClick_middle={handleEvent} />
        <input onClick_enter_right={handleEvent} />
        <input onKeyup_enter={handleEvent} />
        <input onKeyup_tab={handleEvent} />
        <input onKeyup_delete={handleEvent} />
        <input onKeyup_esc={handleEvent} />
        <input onKeyup_space={handleEvent} />
        <input onKeyup_up={handleEvent} />
        <input onKeyup_down={handleEvent} />
        <input onKeyup_left={handleEvent} />
        <input onKeyup_middle={submit} />
        <input onKeyup_middle_self={submit} />
        <input onKeyup_self_enter={handleEvent} />
      </>`,
    )
    expect(code).matchSnapshot()
  })

  test('should error if no expression AND no modifier', () => {
    const onError = vi.fn()
    compile(`<div onClick />`, { onError })
    expect(onError.mock.calls[0][0]).toMatchObject({
      code: ErrorCodes.VOnNoExpression,
      // loc: {
      //   start: {
      //     line: 1,
      //     column: 5,
      //   },
      //   end: {
      //     line: 1,
      //     column: 12,
      //   },
      // },
    })
  })

  test('should NOT error if no expression but has modifier', () => {
    const onError = vi.fn()
    compile(`<div onClick_prevent />`, { onError })
    expect(onError).not.toHaveBeenCalled()
  })

  test('should support multiple modifiers and event options', () => {
    const { code, helpers } = compile(`<div onClick_stop_prevent_capture_once={test}/>`)

    expect(code).toMatchSnapshot()
    expect(helpers).contains('on')
    expect(code).contains(
      `_on(n0, "click", _withModifiers(test, ["stop","prevent"]), {
    capture: true,
    once: true
  })`,
    )
  })

  test('should support multiple events and modifiers options', () => {
    const { code } = compile(`<div onClick_stop={test} onKeyup_enter={test} />`)
    expect(code).toMatchInlineSnapshot(`
      "
        const n0 = t0()
        n0.$evtclick = _withModifiers(test, ["stop"])
        n0.$evtkeyup = _withKeys(test, ["enter"])
        return n0
      "
    `)
    expect(code).contains(`n0.$evtclick = _withModifiers(test, ["stop"])`)
    expect(code).contains(`n0.$evtkeyup = _withKeys(test, ["enter"])`)
  })

  test('should wrap keys guard for keyboard events or dynamic events', () => {
    const { code } = compile(`<div onKeydown_stop_capture_ctrl_a={test}/>`)

    expect(code).matchSnapshot()
  })

  test('should not wrap keys guard if no key modifier is present', () => {
    const { code } = compile(`<div onKeyup_exact={test}/>`)
    expect(code).matchSnapshot()
  })

  test('should wrap keys guard for static key event w/ left/right modifiers', () => {
    const { code } = compile(`<div onKeyup_left={test}/>`)
    expect(code).matchSnapshot()
  })

  test('should transform click.right', () => {
    const { code, delegates } = compile(`<div onClick_right={test}/>`)
    expect(code).toMatchSnapshot()
    expect(delegates).includes('contextmenu')
  })

  test('should transform click.middle', () => {
    const { code, delegates } = compile(`<div onClick_middle={test}/>`)
    expect(code).matchSnapshot()
    expect(delegates).includes('mouseup')
  })

  test('should delegate event', () => {
    const { code, helpers, delegates } = compile(`<div onClick={test}/>`)
    expect(code).matchSnapshot()
    expect(delegates).contains('click')
    expect(helpers).contains('delegateEvents')
  })

  test('should use delegate helper when have multiple events of same name', () => {
    const { code, helpers } = compile(`<div onClick={test} onClick_stop={test} />`)
    expect(helpers).contains('delegate')
    expect(code).toMatchSnapshot()
    expect(code).contains('_delegate(n0, "click", test)')
    expect(code).contains('_delegate(n0, "click", _withModifiers(test, ["stop"]))')
  })

  test('namespace event with Component', () => {
    const { code } = compile(`<Comp onUpdate:modelValue={() => {}} />`)
    expect(code).toMatchSnapshot()
    expect(code).contains('_createComponent(Comp, { "onUpdate:modelValue": () => () => {} }, null, true)')
  })

  test('expression with type', () => {
    const { code } = compile(`<div onClick={handleClick as any} />`)
    expect(code).toMatchInlineSnapshot(`
      "
        const n0 = t0()
        n0.$evtclick = handleClick as any
        return n0
      "
    `)
    expect(code).contains('n0.$evtclick = handleClick')
  })
})

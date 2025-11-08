import { compile, ErrorCodes } from '@vue-jsx-vapor/compiler-rs'
import { describe, expect, test, vi } from 'vitest'

describe('v-html', () => {
  test('should convert v-html to innerHTML', () => {
    const { code, helpers } = compile(`<div v-html={code.value}></div>`)

    expect(helpers).contains('setHtml')
    expect(code).matchSnapshot()
  })

  test('should raise error and ignore children when v-html is present', () => {
    const onError = vi.fn()
    const { code } = compile(`<div v-html={test.value}>hello</div>`, {
      onError,
    })
    expect(code).toMatchSnapshot()

    expect(onError.mock.calls).toMatchObject([[{ code: ErrorCodes.VHtmlWithChildren }]])
  })

  test('should raise error if has no expression', () => {
    const onError = vi.fn()
    const { code } = compile(`<div v-html></div>`, {
      onError,
    })
    expect(code).matchSnapshot()
    expect(onError.mock.calls).toMatchObject([[{ code: ErrorCodes.VHtmlNoExpression }]])
  })
})

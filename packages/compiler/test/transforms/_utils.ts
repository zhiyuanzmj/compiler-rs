import { generate, transform, type CompilerOptions } from '../../src'
import { IRNodeTypes, type RootNode } from '../../src/ir'
import { parseExpression } from '../../src/utils'

export function makeCompile(options: CompilerOptions = {}) {
  return (source: string, overrideOptions: CompilerOptions = {}) => {
    const expression = parseExpression('index.tsx', source)
    const tagType = expression.type
    const children =
      expression.type === 'JSXFragment'
        ? expression.children
        : expression.type === 'JSXElement'
          ? [expression]
          : []
    const ast: RootNode = {
      type: tagType === 'JSXFragment' ? 'JSXFragment' : IRNodeTypes.ROOT,
      children,
      source,
    }
    const ir = transform(ast, {
      filename: 'index.tsx',
      ...options,
      ...overrideOptions,
    }) as any
    return {
      ir,
      ...generate(ir, {
        ...options,
        ...overrideOptions,
      }),
    }
  }
}

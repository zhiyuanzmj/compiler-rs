import type { CodegenOptions } from './generate'
import type { TransformOptions as _TransformOptions } from '@vue-jsx-vapor/compiler-rs'

export type TransformOptions = CodegenOptions & Partial<_TransformOptions>

export { transform } from '@vue-jsx-vapor/compiler-rs'

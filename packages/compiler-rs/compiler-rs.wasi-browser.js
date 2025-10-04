import {
  createOnMessage as __wasmCreateOnMessageForFsProxy,
  getDefaultContext as __emnapiGetDefaultContext,
  instantiateNapiModuleSync as __emnapiInstantiateNapiModuleSync,
  WASI as __WASI,
} from '@napi-rs/wasm-runtime'



const __wasi = new __WASI({
  version: 'preview1',
})

const __wasmUrl = new URL('./compiler-rs.wasm32-wasi.wasm', import.meta.url).href
const __emnapiContext = __emnapiGetDefaultContext()


const __sharedMemory = new WebAssembly.Memory({
  initial: 4000,
  maximum: 65536,
  shared: true,
})

const __wasmFile = await fetch(__wasmUrl).then((res) => res.arrayBuffer())

const {
  instance: __napiInstance,
  module: __wasiModule,
  napiModule: __napiModule,
} = __emnapiInstantiateNapiModuleSync(__wasmFile, {
  context: __emnapiContext,
  asyncWorkPoolSize: 4,
  wasi: __wasi,
  onCreateWorker() {
    const worker = new Worker(new URL('./wasi-worker-browser.mjs', import.meta.url), {
      type: 'module',
    })

    return worker
  },
  overwriteImports(importObject) {
    importObject.env = {
      ...importObject.env,
      ...importObject.napi,
      ...importObject.emnapi,
      memory: __sharedMemory,
    }
    return importObject
  },
  beforeInit({ instance }) {
    for (const name of Object.keys(instance.exports)) {
      if (name.startsWith('__napi_register__')) {
        instance.exports[name]()
      }
    }
  },
})
export default __napiModule.exports
export const DynamicFlag = __napiModule.exports.DynamicFlag
export const findProp = __napiModule.exports.findProp
export const getExpression = __napiModule.exports.getExpression
export const getTextLikeValue = __napiModule.exports.getTextLikeValue
export const IRDynamicPropsKind = __napiModule.exports.IRDynamicPropsKind
export const IRNodeTypes = __napiModule.exports.IRNodeTypes
export const IRSlotType = __napiModule.exports.IRSlotType
export const isBigIntLiteral = __napiModule.exports.isBigIntLiteral
export const isBlockOperation = __napiModule.exports.isBlockOperation
export const isNumericLiteral = __napiModule.exports.isNumericLiteral
export const isStringLiteral = __napiModule.exports.isStringLiteral
export const isTemplate = __napiModule.exports.isTemplate
export const transformVOnce = __napiModule.exports.transformVOnce
export const TS_NODE_TYPES = __napiModule.exports.TS_NODE_TYPES
export const unwrapTSNode = __napiModule.exports.unwrapTSNode

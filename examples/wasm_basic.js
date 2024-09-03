// Load the .wasm file
const wasmCode = await Deno.readFile(
  'target/wasm32-unknown-unknown/release/wasm_basic.wasm'
)

// Instantiate the WebAssembly module
const wasmModule = new WebAssembly.Module(wasmCode)
const wasmInstance = new WebAssembly.Instance(wasmModule)

// Get the exported functions
const {
  mystruct_create,
  mystruct_get_cstring,
  mystruct_get_cstring_len,
  mystruct_set_cstring,
  mystruct_destroy,
  memory,
} = wasmInstance.exports

// Create a new mystruct
const mystruct = mystruct_create()

// Call get_cstring and read the result
const strPtr = mystruct_get_cstring(mystruct)
const strLen = mystruct_get_cstring_len(mystruct)
const strArray = new Uint8Array(memory.buffer, strPtr, strLen)
const str = new TextDecoder().decode(strArray)
console.log('Current string:', str)

mystruct_destroy(mystruct)

// Set a new string
// const newStr = 'Hello, WebAssembly!'
// const encoder = new TextEncoder()
// const newStrArray = encoder.encode(newStr)
// const newStrPtr = mystruct_set_cstring(mystruct, newStrArray.length)
// new Uint8Array(memory.buffer).set(newStrArray, newStrPtr)

// Verify the new string
const newStrPtr2 = mystruct_get_cstring(mystruct)
const newStrLen = mystruct_get_cstring_len(mystruct)
const newStrArray2 = new Uint8Array(memory.buffer, newStrPtr2, newStrLen)
const newStr2 = new TextDecoder().decode(newStrArray2)
console.log('New string:', newStr2)

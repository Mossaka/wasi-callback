# wasi-callback

This repo demonstrate how to implement a callback function in a wasm module. It uses wasm component model and `wit-bindgen` to generate bindings for rust code.

- The guest wasm module is in `./demo/` directory. It exports the `event-handler` interface and imports `exec` interface. It will be compiled to `./target/wasm32-wasi/release/demo.wasm`.
- The host is written in Rust using wasmtime SDK, and it exports the `exec` interface for the guest and imports the `event-handler` interface. It will be compiled to `./target/release/wasi-callback` as a single binary. 

The invocation of the guest wasm module is as follows:
1. The host instantiates the guest wasm module.
2. The guest module prints the message to stdout.
3. Th guest module invokes `exec()` function in the host.
4. The host prints the message to stdout.
5. The host invokes `event-handler()` function in the guest.
6. The guest finishes execution.

## Build and Run
run `make build && make run`

## Future
- [x] Think about a way to remove the `unsafe` block.
- [x] Make it thread-safe

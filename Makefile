build: 
	cargo build --release
	cargo build --target=wasm32-wasi --release --manifest-path ./demo/Cargo.toml

run:
	./target/release/wasi-callback

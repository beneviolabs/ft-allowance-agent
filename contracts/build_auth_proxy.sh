# https://github.com/near/near-workspaces-js/issues/225#issuecomment-1853577966

echo "Running cargo formatter "
cargo fmt


env RUSTFLAGS="-Z unstable-options" cargo +nightly near build non-reproducible-wasm --no-abi --no-wasmopt

# Run wasm-opt manually with bulk memory and sign extension enabled
echo "Running wasm-opt with bulk memory and sign extension support"
wasm-opt --enable-bulk-memory --enable-sign-ext -O target/wasm32-unknown-unknown/release/proxy_contract.wasm -o target/near/proxy_contract.wasm

# https://github.com/near/near-workspaces-js/issues/225#issuecomment-1853577966

echo "Running cargo formatter "
cargo fmt


env RUSTFLAGS="-Z unstable-options -C target-feature=+bulk-memory" cargo +nightly near build non-reproducible-wasm --no-abi --no-wasmopt

# https://github.com/near/near-workspaces-js/issues/225#issuecomment-1853577966

env RUSTFLAGS='-Ctarget-cpu=mvp' cargo +nightly build -Zbuild-std=panic_abort,std --target=wasm32-unknown-unknown --release

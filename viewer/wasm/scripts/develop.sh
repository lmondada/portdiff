# A script to watch for changes to the `wasm` directory and run `wasm-pack` to 
# rebuild the bundle.
# If you are running `npm run develop` on the React app, it will automatically 
# reload when the build is created.
# 
# Note: This requires:
# - wasm-pack https://github.com/rustwasm/wasm-pack
# - cargo-watch https://github.com/watchexec/cargo-watch
cargo watch -- wasm-pack build --out-dir ../wasm-build
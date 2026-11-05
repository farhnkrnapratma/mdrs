#[cfg(all(feature = "napi", not(target_arch = "wasm32")))]
mod node;
#[cfg(feature = "wasm")]
mod wasm;

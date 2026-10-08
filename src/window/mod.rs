#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(not(target_arch = "wasm32"))]
mod windows;

#[cfg(target_arch = "wasm32")]
pub use web::*;
#[cfg(not(target_arch = "wasm32"))]
pub use windows::*;

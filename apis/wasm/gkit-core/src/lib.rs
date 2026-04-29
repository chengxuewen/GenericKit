use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn hello() {
    gkit_core::core_hello();
}

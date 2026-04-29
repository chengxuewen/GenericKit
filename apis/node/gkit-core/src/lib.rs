use napi_derive::napi;

#[napi]
pub fn hello() {
    gkit_core::core_hello();
}

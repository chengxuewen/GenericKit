use gkit_core;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_core_hello() {
    gkit_core::core_hello();
}

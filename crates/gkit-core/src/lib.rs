#[cfg(feature = "plugin")]
pub mod plugin;
pub mod version;

pub fn core_hello() {
    println!("core_hello!");
}

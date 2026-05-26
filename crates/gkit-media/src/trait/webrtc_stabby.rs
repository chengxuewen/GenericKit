use stabby::string::String as StabbyString;

#[stabby::stabby(checked)]
pub trait IStablePeerConnectionFactory {
    extern "C" fn backend_name(&self) -> StabbyString;
}

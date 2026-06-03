#[macro_export]
macro_rules! gkit_register_rtc_backend {
    ($name:expr, $factory:ty) => {
        #[doc(hidden)]
        #[cfg_attr(not(test), ::ctor::ctor)]
        fn __gkit_rtc_register() {
            $crate::protocols::rtc::peer::engine::RtcEngine::register(
                $name,
                || Box::new(<$factory as Default>::default()),
            );
        }
    };
}

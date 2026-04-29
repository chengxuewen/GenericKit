use gkit_media;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gkit_media_hello() {
    gkit_media::media_hello();
}

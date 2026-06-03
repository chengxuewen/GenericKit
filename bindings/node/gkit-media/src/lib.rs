use napi_derive::napi;

#[napi]
pub fn hello() {
    gkit_media::media_hello();
}

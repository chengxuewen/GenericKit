#[tokio::test]
async fn google_backend_registered() {
    use gkit_media::protocols::rtc::client::engine::RtcEngine;

    let types = RtcEngine::registered_types();
    assert!(
        types.contains(&"google".to_string()),
        "google backend not in registered types: {types:?}"
    );
}

#[tokio::test]
#[ignore = "pre-built libwebrtc binary crashes on macOS during PCF init (NSString+StdString ObjC category issue)"]
async fn create_and_close_peer_connection() {
    use gkit_media::protocols::rtc::client::core::ConnectionState;
    use gkit_media::protocols::rtc::client::engine::RtcEngine;

    let factory = RtcEngine::create("google").expect("google backend not registered");

    let mut pc = factory
        .create_peer_connection()
        .expect("create peer connection");

    let state = pc.connection_state();
    assert!(
        matches!(state, ConnectionState::New | ConnectionState::Connecting),
        "unexpected initial state: {state:?}"
    );

    pc.close().expect("close peer connection");
    assert_eq!(pc.connection_state(), ConnectionState::Closed);
}

#[tokio::test]
#[ignore = "pre-built libwebrtc binary crashes on macOS during PCF init (NSString+StdString ObjC category issue)"]
async fn create_factory_twice() {
    use gkit_media::protocols::rtc::client::engine::RtcEngine;

    let f1 = RtcEngine::create("google").expect("first factory");
    let f2 = RtcEngine::create("google").expect("second factory");

    let mut pc1 = f1.create_peer_connection().expect("pc1");
    let mut pc2 = f2.create_peer_connection().expect("pc2");

    pc1.close().unwrap();
    pc2.close().unwrap();
}


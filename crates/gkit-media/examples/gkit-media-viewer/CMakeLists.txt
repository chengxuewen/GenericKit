include(GKitCargoExample)

gkit_cargo_add_example(
    NAME gkit-media-viewer
    CRATE gkit-media
    ASSETS "${CMAKE_SOURCE_DIR}/assets"
    FOLDER gkit_media/examples
    RELEASE
)

# GenericKit Status

**Last Updated**: 2026-05-26
**Active Session**: Plugin Architecture Migration (P0-P5) + CMake Integration

## Plugin Architecture

| Phase | Status | Description |
|-------|--------|-------------|
| P0 | ✅ | gkit-core plugin loader (PluginLib, PluginLoader, PluginDiscovery) |
| P1 | ✅ | stabby types (StableVideoFrame, IStableVideoSink, IStablePeerConnectionFactory) |
| P2 | ✅ | gkit-media PluginRegistry<T> |
| P3 | ✅ | First cdylib plugin (gkit-plugin-webrtc-libwebrtc) |
| P4 | ✅ | RtcEngine PluginRegistry integration + dynamic plugin loading |
| P5 | ✅ | WASM web-sys plugin (rlib static linking) |

## Crates

| Crate | Type | Status |
|-------|------|--------|
| gkit-core | rlib | Plugin infrastructure |
| gkit-media | rlib | Core traits + engine (backend-free) |
| gkit-plugin-webrtc-libwebrtc | cdylib | Native libwebrtc plugin |
| gkit-plugin-webrtc-web-sys | rlib | WASM browser WebRTC plugin |

## CMake Plugin Build

| Feature | Status |
|--------|--------|
| GKitCargoPlugin.cmake | ✅ `gkit_cargo_add_plugin()` + `gkit_cargo_setup_plugins()` |
| POST_BUILD copy to build/plugins/ | ✅ via `add_custom_target(copy-plugin-*)` DEPENDS cargo-build_ |
| FOLDER property | ✅ main target + all Corrosion utility targets (12 prefix variants) |
| install() rule | ✅ `install(FILES ... DESTINATION lib/plugins/<category>/)` |
| Configure re-run safety | ✅ `if(NOT TARGET)` guards on target creation |

## Test Suite

| Component | Pass | Ignored |
|-----------|------|---------|
| gkit-core | 19 | 5 |
| gkit-media lib | 13 | 0 |
| gkit-media webrtc | 21 | 6 |
| Plugin loading | 4 | 2 |

## Git State

- 27 commits on main
- Working tree: clean
- Latest: CMake plugin build + install + FOLDER verified

## Remaining Work

- webrtc-rs plugin (temporarily postponed)
- Full IStablePeerConnection stabby trait
- P2P/ICE tests with real backends (current: #[ignore])

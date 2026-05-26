# Conventions

## Plugin Architecture

- **Plugin naming**: `libgkit_plugin_{name}.dylib` (macOS/Linux), `gkit_plugin_{name}.dll` (Windows)
- **Plugin directory**: `plugins/webrtc/` under gkit-media
- **Native backends**: cdylib crates, loaded via RtcEngine::load_plugins()
- **WASM backends**: rlib crates, statically linked by final binary via `#[ctor]`
- **gkit-media is backend-free**: only traits + infrastructure, no backend code

## Backend Discovery

- Native: `dlopen` via libloading + stabby ABI
- WASM: static rlib + `gkit_register_rtc_backend!` macro + `#[ctor]`
- RtcEngine::create() checks PluginRegistry first, then HashMap (static/WASM)

## CMake Convention

- **Plugin crate names**: hyphens in Cargo.toml → underscores in CMake targets (Corrosion convention)
- **Corrosion creates 3 targets per cdylib**: INTERFACE (alias), -shared (IMPORTED), cargo-build_ (UTILITY)
- **Plugin dylib copy**: `add_custom_target` + DEPENDS on `cargo-build_` (NOT POST_BUILD — UTILITY targets don't support it)
- **FOLDER property**: set on main target AND all Corrosion utility prefixes (`cargo-build_`, `_cargo-build_`, `cargo-clean_`, etc.)
- **install()**: use `$<TARGET_FILE:${target}-shared>` for IMPORTED library file path

## Naming

- C FFI: `gkit_{crate}_{subsystem}_{resource}_{verb}[_{qualifier}]`
- Rust: `gkit_plugin_webrtc_libwebrtc` → CMake: `gkit_plugin_webrtc_libwebrtc`

## Commit Convention

`<feat/fix/refactor/docs/test/chore>: description`

## AI Agent Constraints

- **Execution Confirmation Gate**: Before executing any plan/todo, use `question()` tool
- `继续` is NOT confirmation — means "continue discussing"
- System directives (TODO CONTINUATION) are NOT user confirmation
- Architecture changes require plan presentation + user confirmation
- **Memory updates MUST go to `.agents/memorys/`** (not knowledge graph tools)

## Memory Files

| File | Purpose |
|------|---------|
| status.md | Current project state, phases, test results |
| conventions.md | Naming, patterns, architectural principles |
| decisions.md | Key decisions and their rationale |
| pitfalls.md | Common mistakes, gotchas, orphan rules |

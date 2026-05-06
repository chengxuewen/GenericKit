# AI Development Guide

## Build

```bash
cargo build                                                  # Rust workspace
cargo test                                                   # Rust tests
cmake -B build-auto -S . -DGKIT_BUILD_TESTS=ON               # Full configure
cmake --build build-auto                                     # Full build
ctest --test-dir build-auto                                  # CTest tests
```

## Architecture

```text
crates/     # Rust crates (gkit-core, gkit-media, gkit-network, ...)
apis/       # FFI bindings (c/cpp/python/wasm/node/csharp/flutter)
cmake/      # CMake modules
tools/      # CLI tools
```

## Key Rules

- **C FFI naming**: `gkit_<crate>_<subsystem>_<resource>_<verb>[_<qualifier>]`
- **Commit**: `<feat/fix/refactor/docs/test/chore>: description`
- **Version bump**: Edit `VERSION.txt` → re-run CMake → auto-syncs to `Cargo.toml`
- **New module**: Add to `Cargo.toml` members → add to CMake `_gkit_corrosion_crates` → wire tests

## .agents

The `.agents/` directory contains AI development resources:

```text
.agents/
├── rules/        # Coding standards for AI assistants
│   ├── common/   # Language-agnostic principles
│   └── ...       # Language-specific rules (rust, cpp, python, ...)
└── prompts/      # Task plans and system prompts
    ├── tasks/    # Feature task lists
    └── systems/  # System-level prompts
```

Rules are project-local and read directly from `.agents/rules/` — no external installation needed.

See [.agents/rules/README.md](.agents/rules/README.md) for full documentation.

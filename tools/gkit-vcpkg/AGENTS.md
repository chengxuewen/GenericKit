# gkit-vcpkg AGENTS.md

**Parent:** [../../AGENTS.md](../../AGENTS.md) &mdash; root conventions, build, anti-patterns

## OVERVIEW

gkit-vcpkg is a Rust CLI tool for managing vcpkg C/C++ dependencies within GenericKit. Uses clap for argument parsing with 8 subcommands.

## STRUCTURE

```
tools/gkit-vcpkg/
├── src/
│   └── main.rs           # CLI entry point (clap, 8 subcommands)
├── Cargo.toml            # Dependencies: clap, serde, etc.
└── CMakeLists.txt        # CMake custom target for building via Cargo
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add new subcommand | `src/main.rs` | Define clap SubCommand + match arm |
| Change vcpkg interaction | `src/main.rs` | Subcommand handler functions |
| Change vcpkg install logic | `src/main.rs` | `gkit_vcpkg_install()` function |
| Update dependencies | `Cargo.toml` | Add clap features or new crates |

## SUBCOMMANDS

| Command | Purpose |
|---------|---------|
| `init` | Initialize vcpkg (clone + bootstrap) |
| `install <package>` | Install a C/C++ package via vcpkg |
| `find <package>` | Search for available packages |
| `list` | List installed packages |
| `remove <package>` | Remove an installed package |
| `export` | Export installed packages for offline use |
| `cargo-config` | Generate Cargo config for vcpkg paths |
| `cmake-toolchain` | Output CMake toolchain path for vcpkg |

## CONVENTIONS

- **clap builder pattern**: Uses the builder/command pattern (not derive macros)
- **Error handling**: Uses `anyhow::Result` for application-level errors
- **Output**: Uses `println!` for user-facing output, `eprintln!` for errors

## BUILD

```bash
# Standalone
cargo build -p gkit-vcpkg
cargo run -p gkit-vcpkg -- install <package>

# Via CMake (as part of full build)
cmake --build build-auto --target gkit_vcpkg
```

## NOTES

- vcpkg is **not a submodule** — it's cloned fresh during `init` if not present
- vcpkg maintains **two copies**: `vcpkg/` (working copy) + `vcpkg-tools/` (clean tool copy)
- The tool wraps `vcpkg` CLI commands internally — it does not implement vcpkg logic itself

// gkit-vcpkg — vcpkg integration tool for GenericKit
//
// Inspired by:
//   - https://github.com/mcgoo/vcpkg-rs   (Rust library for vcpkg)
//   - https://github.com/mcgoo/cargo-vcpkg (Cargo subcommand)
//   - OpenCTK cmake/InstallVcpkg.cmake      (CMake integration)
//
// Usage:
//   gkit-vcpkg find <package>             # locate vcpkg-installed package
//   gkit-vcpkg install <package>          # install package via vcpkg
//   gkit-vcpkg export <package>           # export for offline use
//   gkit-vcpkg cargo-config              # generate .cargo/config for vcpkg
//   gkit-vcpkg cmake-toolchain            # print cmake toolchain path

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser)]
#[command(name = "gkit-vcpkg", version, about = "GenericKit vcpkg integration tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to vcpkg root (default: VCPKG_ROOT env or ../vcpkg)
    #[arg(long, global = true)]
    vcpkg_root: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Find an installed vcpkg package
    Find {
        package: String,
        /// Triplet (e.g., x64-osx, arm64-osx)
        #[arg(long, default_value_t = default_triplet())]
        triplet: String,
    },

    /// Install a package via vcpkg
    Install {
        package: String,
        #[arg(long)]
        triplet: Option<String>,
        /// Extra features/components
        #[arg(long)]
        features: Vec<String>,
        /// Do not recurse dependencies
        #[arg(long)]
        no_recurse: bool,
    },

    /// Export an installed package for offline use
    Export {
        package: String,
        /// Output directory
        #[arg(long, default_value = "3rdparty/vcpkg")]
        output_dir: PathBuf,
        #[arg(long)]
        triplet: Option<String>,
        /// Raw export format
        #[arg(long)]
        raw: bool,
    },

    /// Generate Cargo config for vcpkg integration
    CargoConfig {
        /// Output path (.cargo/config.toml)
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Print CMake toolchain file path
    CmakeToolchain,

    /// Initialize vcpkg (clone + bootstrap)
    Init {
        /// GitHub URL (default: microsoft/vcpkg)
        #[arg(long, default_value = "https://github.com/microsoft/vcpkg.git")]
        url: String,
    },

    /// List all installed packages
    List {
        #[arg(long)]
        triplet: Option<String>,
    },
}

fn default_triplet() -> String {
    if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") { "arm64-osx".into() }
        else { "x64-osx".into() }
    } else if cfg!(target_os = "windows") {
        if cfg!(target_arch = "x86_64") { "x64-windows-static-md".into() }
        else { "x64-windows-static-md".into() }
    } else {
        if cfg!(target_arch = "x86_64") { "x64-linux".into() }
        else { "arm64-linux".into() }
    }
}

fn resolve_vcpkg_root(cli_root: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = cli_root { return Ok(p); }
    if let Ok(p) = std::env::var("VCPKG_ROOT") { return Ok(PathBuf::from(p)); }
    let exe = std::env::current_exe()?;
    for ancestor in exe.ancestors() {
        let candidate = ancestor.join("vcpkg");
        if candidate.join(".git").exists() { return Ok(candidate); }
    }
    bail!("vcpkg not found. Set VCPKG_ROOT or use --vcpkg-root")
}

fn vcpkg_executable(root: &Path) -> PathBuf {
    if cfg!(windows) { root.join("vcpkg.exe") } else { root.join("vcpkg") }
}

fn run_vcpkg(root: &Path, args: &[&str]) -> Result<String> {
    let exe = vcpkg_executable(root);
    if !exe.exists() {
        bail!("vcpkg executable not found at {}. Run 'gkit-vcpkg init' first.", exe.display());
    }
    let output = Command::new(&exe)
        .args(args)
        .current_dir(root)
        .output()
        .context("Failed to run vcpkg")?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        eprintln!("{stderr}");
        bail!("vcpkg command failed");
    }
    if !stderr.is_empty() { eprintln!("{stderr}"); }
    Ok(stdout)
}

fn get_triplet(cli_triplet: Option<String>) -> String {
    cli_triplet.unwrap_or_else(default_triplet)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let vcpkg_root = resolve_vcpkg_root(cli.vcpkg_root)?;

    match cli.command {
        Commands::Find { package, triplet } => cmd_find(&vcpkg_root, &package, &triplet),
        Commands::Install { package, triplet, features, no_recurse } => {
            cmd_install(&vcpkg_root, &package, triplet, &features, no_recurse)
        }
        Commands::Export { package, output_dir, triplet, raw } => {
            cmd_export(&vcpkg_root, &package, &output_dir, triplet, raw)
        }
        Commands::CargoConfig { output } => cmd_cargo_config(&vcpkg_root, output),
        Commands::CmakeToolchain => cmd_cmake_toolchain(&vcpkg_root),
        Commands::Init { url } => cmd_init(&vcpkg_root, &url),
        Commands::List { triplet } => cmd_list(&vcpkg_root, triplet),
    }
}

// ============================================================================
// Commands
// ============================================================================

fn cmd_init(root: &Path, url: &str) -> Result<()> {
    if root.join(".git").exists() {
        println!("vcpkg already exists at {}", root.display());
        if !vcpkg_executable(root).exists() {
            println!("Bootstrapping vcpkg...");
            bootstrap_vcpkg(root)?;
        }
        return Ok(());
    }

    println!("Cloning vcpkg from {}...", url);
    if let Some(parent) = root.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let status = Command::new("git")
        .args(["clone", url, "--depth", "1"])
        .arg(root)
        .status()
        .context("git clone failed")?;
    if !status.success() { bail!("git clone failed"); }

    println!("Bootstrapping vcpkg...");
    bootstrap_vcpkg(root)?;
    println!("vcpkg ready at {}", root.display());
    Ok(())
}

fn bootstrap_vcpkg(root: &Path) -> Result<()> {
    let script = if cfg!(windows) { "bootstrap-vcpkg.bat" } else { "bootstrap-vcpkg.sh" };
    let cmd = if cfg!(windows) { script.to_string() } else { format!("./{}", script) };
    let status = if cfg!(windows) {
        Command::new("cmd").args(["/C", &cmd]).current_dir(root).status()?
    } else {
        Command::new("sh").arg(&cmd).current_dir(root).status()?
    };
    if !status.success() { bail!("vcpkg bootstrap failed"); }
    Ok(())
}

fn cmd_find(root: &Path, package: &str, triplet: &str) -> Result<()> {
    let list = run_vcpkg(root, &["list", &format!("{}:{}", package, triplet)])?;
    if list.trim().is_empty() {
        println!("Package '{}:{}' not found", package, triplet);
        std::process::exit(1);
    }
    println!("{}", list.trim());
    Ok(())
}

fn cmd_install(root: &Path, package: &str, triplet: Option<String>, features: &[String], no_recurse: bool) -> Result<()> {
    let triplet = get_triplet(triplet);
    let spec = if features.is_empty() {
        format!("{}:{}", package, triplet)
    } else {
        format!("{}[{}]:{}", package, features.join(","), triplet)
    };

    let mut args = vec!["install", &spec];
    if !no_recurse { args.push("--recurse"); }

    println!("Installing {} ...", spec);
    run_vcpkg(root, &args)?;
    println!("Package {} installed successfully", spec);
    Ok(())
}

fn cmd_export(root: &Path, package: &str, output_dir: &Path, triplet: Option<String>, raw: bool) -> Result<()> {
    let triplet = get_triplet(triplet);
    let spec = format!("{}:{}", package, triplet);

    std::fs::create_dir_all(output_dir)?;

    let mut args = vec!["export", &spec, "--output-dir"];
    let output_dir_s = output_dir.to_string_lossy();
    args.push(&output_dir_s);
    if raw { args.push("--raw"); }

    println!("Exporting {} to {}...", spec, output_dir.display());
    run_vcpkg(root, &args)?;
    println!("Package {} exported to {}", spec, output_dir.display());
    Ok(())
}

fn cmd_cargo_config(root: &Path, output: Option<PathBuf>) -> Result<()> {
    let installed_dir = root.join("installed").join(default_triplet());
    let lib_dir = installed_dir.join("lib");
    let _include_dir = installed_dir.join("include");

    let config = format!(
        r#"[target.{}]
rustc-link-search = ["{}"]
rustc-link-lib = ["static=vcpkg"]

[env]
VCPKG_ROOT = "{}"
VCPKG_INSTALLED = "{}"
"#,
        target_triple(),
        lib_dir.display(),
        root.display(),
        installed_dir.display(),
    );

    if let Some(out) = output {
        std::fs::create_dir_all(out.parent().unwrap())?;
        std::fs::write(&out, &config)?;
        println!("Cargo config written to {}", out.display());
    } else {
        println!("{}", config);
    }
    Ok(())
}

fn cmd_cmake_toolchain(root: &Path) -> Result<()> {
    let toolchain = root.join("scripts/buildsystems/vcpkg.cmake");
    if toolchain.exists() {
        println!("{}", toolchain.display());
        println!("\nUsage: cmake -DCMAKE_TOOLCHAIN_FILE=\"{}\" ...", toolchain.display());
    } else {
        bail!("CMake toolchain not found at {}", toolchain.display());
    }
    Ok(())
}

fn cmd_list(root: &Path, triplet: Option<String>) -> Result<()> {
    let mut args = vec!["list"];
    let triplet_s;
    if let Some(t) = triplet {
        triplet_s = format!("*:{}", t);
        args.push(&triplet_s);
    }
    run_vcpkg(root, &args)?;
    Ok(())
}

fn target_triple() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

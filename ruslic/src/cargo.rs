use serde::Deserialize;

fn main() {
    if let Err(code) = process(std::env::args().skip(1)) {
        std::process::exit(code);
    }
}

fn process<I>(args: I) -> Result<(), i32>
where
    I: Iterator<Item = String>,
{
    let mut russol_rustc_path = std::env::current_exe()
        .expect("current executable path invalid")
        .with_file_name("ruslic");
    if cfg!(windows) {
        russol_rustc_path.set_extension("exe");
    }

    // Remove the "russol" argument when `cargo-russol` is invoked as
    // `cargo --cflag russol` (note the space in `cargo russol` rather than a `-`)
    let args: Vec<_> = args.skip_while(|arg| arg == "russol").collect();

    let exit_status = std::process::Command::new("cargo")
        .arg("check")
        .args(&args)
        // Otherwise `rust-analyzer` might run `cargo check` and so this one would do nothing (cached result)
        .env("CARGO_INCREMENTAL", "false")
        .env("RUST_TOOLCHAIN", get_rust_toolchain_channel())
        .env("RUSTUP_TOOLCHAIN", get_rust_toolchain_channel())
        .env("RUSTC_WRAPPER", russol_rustc_path)
        .env("RUSLIC_OPTIMISTICALLY_ALLOW_PRIVATE_TYPES", "true")
        .env("RUSLIC_SUBST_RESULT", "true")
        .env("RUSLIC_SUMMARISE", "true")
        .status()
        .expect("could not run cargo");

    if !exit_status.success() {
        return Err(exit_status.code().unwrap_or(-1));
    }

    let path = args.iter().find(|arg| arg.starts_with("--manifest-path="));
    // Run fmt after `RUSLIC_SUBST_RESULT`
    let exit_status = std::process::Command::new("cargo")
        .arg("fmt")
        .args(path)
        .status()
        .expect("could not run cargo");
    if !exit_status.success() {
        Ok(())
    } else {
        Err(exit_status.code().unwrap_or(-1))
    }
}

pub fn get_rust_toolchain_channel() -> String {
    #[derive(Deserialize)]
    struct RustToolchainFile {
        toolchain: RustToolchain,
    }

    #[derive(Deserialize)]
    struct RustToolchain {
        channel: String,
        #[allow(dead_code)]
        components: Option<Vec<String>>,
    }

    let content = include_str!("../../rust-toolchain.toml");
    // Be ready to accept TOML format
    // See: https://github.com/rust-lang/rustup/pull/2438
    if content.starts_with("[toolchain]") {
        let rust_toolchain: RustToolchainFile =
            toml::from_str(content).expect("failed to parse rust-toolchain file");
        rust_toolchain.toolchain.channel
    } else {
        content.trim().to_string()
    }
}

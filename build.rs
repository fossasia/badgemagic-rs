fn main() {
    #[cfg(feature = "cli")]
    cli::generate_version_info();
}

#[cfg(feature = "cli")]
mod cli {
    use std::{env, fs, path::PathBuf, process::Command, str};

    pub fn generate_version_info() {
        let pkg_version = env::var("CARGO_PKG_VERSION").expect("missing package version");
        let git_version = git_version();
        let git_prefix = if git_version.is_some() { "commit-" } else { "" };
        let git_version = git_version.as_deref().unwrap_or("unknown");
        let version = format!("{pkg_version}-git.{git_prefix}{git_version}");

        let out: PathBuf = env::var_os("OUT_DIR").expect("build output path").into();
        fs::write(
            out.join("cli.rs"),
            format!("pub const VERSION: &str = {version:?};\n"),
        )
        .expect("write cli.rs");
    }

    fn git_version() -> Option<String> {
        let output = Command::new("git")
            .arg("describe")
            .arg("--always")
            .arg("--dirty=-modified")
            .output()
            .ok()?;
        if output.status.success() {
            Some(str::from_utf8(&output.stdout).ok()?.trim().into())
        } else {
            None
        }
    }
}

//! Management and invocation of external tools, that are required to build projects.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};
use cargo_lock::Lockfile;
use directories::ProjectDirs;
use once_cell::sync::OnceCell;
use serde::Deserialize;

/// Wrapper around [rustup](https://rustup.rs/), to manage toolchain and target installations.
pub struct Rustup {}

impl Rustup {
    const WASM_TARGET: &str = "wasm32-unknown-unknown";

    fn bin_path() -> Result<&'static Path> {
        static BIN_PATH: OnceCell<PathBuf> = OnceCell::new();

        BIN_PATH
            .get_or_try_init(|| which::which("rustup").map_err(Into::into))
            .map(|path| path.as_path())
    }

    pub fn check_wasm_target() -> Result<bool> {
        let output = Command::new(Self::bin_path()?)
            .args(["target", "list", "--installed"])
            .output()?;

        if !output.status.success() {
            bail!(
                "failed checking installed targets: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let stdout = String::from_utf8(output.stdout)?;

        Ok(stdout.lines().any(|line| line == Self::WASM_TARGET))
    }

    pub fn install_wasm_target() -> Result<()> {
        let output = Command::new(Self::bin_path()?)
            .args(["target", "add", Self::WASM_TARGET])
            .output()?;

        if !output.status.success() {
            bail!(
                "failed installing wasm target: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }
}

/// Wrapper around [cargo](https://doc.rust-lang.org/cargo), to compile the Rust code into WASM
/// binaries.
pub struct Cargo {
    /// Location of the workspace root, which can be the project path itself if it's at the top.
    workspace_dir: PathBuf,
    /// Location of the `target` directly usually located at the workspace root. May be changed by
    /// user configuration.
    target_dir: PathBuf,
}

impl Cargo {
    const WASM_TARGET: &str = "wasm32-unknown-unknown";

    fn bin_path() -> Result<&'static Path> {
        static BIN_PATH: OnceCell<PathBuf> = OnceCell::new();

        BIN_PATH
            .get_or_try_init(|| which::which("cargo").map_err(Into::into))
            .map(|path| path.as_path())
    }

    /// Create a new instance for the given project. This will directly locate the workspace root
    /// and target directory for later use.
    pub fn new(working_dir: &Path) -> Result<Self> {
        #[derive(Deserialize)]
        struct Metadata {
            target_directory: PathBuf,
            workspace_root: PathBuf,
        }

        let mut cmd = Command::new(Self::bin_path()?);
        cmd.current_dir(working_dir);
        cmd.args(["metadata", "--format-version", "1"]);

        let output = cmd.output()?;

        if !output.status.success() {
            bail!(
                "failed running cargo:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let meta = serde_json::from_slice::<Metadata>(&output.stdout)
            .context("failed parsing Cargo metadata")?;

        Ok(Self {
            workspace_dir: meta.workspace_root,
            target_dir: meta.target_directory,
        })
    }

    pub fn run(&self, working_dir: &Path, release: bool, profile: &str) -> Result<()> {
        let mut cmd = Command::new(Self::bin_path()?);
        cmd.current_dir(working_dir);
        cmd.args([
            "build",
            "--color",
            "always",
            "--target",
            Self::WASM_TARGET,
            "--target-dir",
        ]);
        cmd.arg(self.target_dir.join("wazzup"));

        if release {
            cmd.args(["--profile", profile]);
        }

        let output = cmd.output()?;

        if !output.status.success() {
            bail!(
                "failed running cargo:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Get the directory of the workspace root, which is where most mandatory files are located
    /// (like the `Cargo.lock`).
    pub fn workspace_dir(&self) -> &Path {
        &self.workspace_dir
    }

    /// Output directory for compilation artifacts.
    pub fn target_dir(&self) -> &Path {
        &self.target_dir
    }
}

/// Wrapper around [wasm-bindgen](https://rustwasm.github.io/docs/wasm-bindgen/), to generate
/// needed JavaScript glue to for loading in the browser.
pub struct WasmBindgen {
    /// Version of `wasm-bindgen`, as discovered from the project's `Cargo.lock` file.
    version: semver::Version,
    /// Absolute path to the binary.
    path: PathBuf,
}

impl WasmBindgen {
    /// Find the `wasm-bingen` version from a project's Cargo.lock file.
    pub fn find_version(lockfile: impl AsRef<Path>) -> Result<semver::Version> {
        Ok(Lockfile::load(lockfile)?
            .packages
            .into_iter()
            .find(|p| p.name.as_str() == "wasm-bindgen")
            .context("no wasm-bindgen dependency")?
            .version)
    }

    /// Create a new instance for the specific version of `wasm-bindgen`. This binary for this
    /// version may or may not exist on the system.
    pub fn new(version: semver::Version) -> Result<Self> {
        let path = ProjectDirs::from("rocks", "dnaka91", "wazzup")
            .context("failed finding project dirs")?
            .cache_dir()
            .join(format!("wasm-bindgen/{version}/wasm-bindgen"));

        Ok(Self { version, path })
    }

    /// Check whether the current version of `wasm-bindgen` is locally installed.
    pub fn installed(&self) -> bool {
        self.path.exists()
    }

    /// Installed the version of `wasm-bindgen` as represented by this instance.
    ///
    /// The binary will be installed with `cargo install`, into a temporary directory, and then
    /// copied over to the application's cache folder. That allows to have multiple versions
    /// installed for re-use, and not interefere with the potentially system-installed version.
    pub fn install(&self) -> Result<()> {
        let tempdir = tempfile::tempdir()?;

        let output = Command::new(Cargo::bin_path()?)
            .args(["install", "--root"])
            .arg(tempdir.path())
            .args([
                "--no-track",
                "--bin",
                "wasm-bindgen",
                "--version",
                &self.version.to_string(),
                "wasm-bindgen-cli",
            ])
            .output()?;

        if !output.status.success() {
            bail!(
                "failed building wasm-bindgen (v{}): {}",
                self.version,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }

        fs::copy(tempdir.path().join("bin/wasm-bindgen"), &self.path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perm = fs::metadata(&self.path)?.permissions();
            let mode = perm.mode();

            if mode & 0o100 == 0 {
                perm.set_mode(mode | 0o100);
                fs::set_permissions(&self.path, perm)?;
            }
        }

        Ok(())
    }

    pub fn run(&self, target: &Path, out: &Path) -> Result<()> {
        let output = Command::new(&self.path)
            .args([
                "--target",
                "web",
                "--no-typescript",
                "--omit-default-module-path",
                "--out-dir",
            ])
            .args([out, target])
            .output()?;

        if !output.status.success() {
            bail!(
                "failed running wasm-bindgen: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }
}

/// Wrapper around [wasm-opt](https://github.com/WebAssembly/binaryen), to further optimize WASM
/// binaries for speed or size.
pub struct WasmOpt {}

impl WasmOpt {
    fn bin_path() -> Result<&'static Path> {
        static BIN_PATH: OnceCell<PathBuf> = OnceCell::new();

        BIN_PATH
            .get_or_try_init(|| which::which("wasm-opt").map_err(Into::into))
            .map(|path| path.as_path())
    }

    pub fn run(target: &Path) -> Result<()> {
        let output = Command::new(Self::bin_path()?)
            .args(["-O4", "--output"])
            .args([target, target])
            .output()?;

        if !output.status.success() {
            bail!(
                "failed running wasm-opt: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }
}

/// Wrapper around [dart-sass](https://github.com/sass/dart-sass), to compile SASS/SCSS/CSS files
/// into optimized CSS stylesheets.
pub struct Sass {}

impl Sass {
    fn bin_path() -> Result<&'static Path> {
        static BIN_PATH: OnceCell<PathBuf> = OnceCell::new();

        BIN_PATH
            .get_or_try_init(|| which::which("sass").map_err(Into::into))
            .map(|path| path.as_path())
    }

    pub fn run(target: &Path, out: &Path, release: bool) -> Result<()> {
        let mut cmd = Command::new(Self::bin_path()?);

        if release {
            cmd.args(["--style", "compressed"]);
        }

        let output = cmd.arg("--no-source-map").args([target, out]).output()?;

        if !output.status.success() {
            bail!(
                "failed running sass: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rustup_check_wasm_target() -> Result<()> {
        assert!(Rustup::check_wasm_target()?);
        Ok(())
    }

    #[test]
    #[cfg(not(coverage))]
    fn run_cargo_bindgen_opt() -> Result<()> {
        let dir = tempfile::TempDir::new()?;
        let project = dir.path().join("temp");

        let status = Command::new(Cargo::bin_path()?)
            .current_dir(dir.path())
            .args(["new", "temp"])
            .output()?
            .status;
        assert!(status.success());

        let status = Command::new(Cargo::bin_path()?)
            .current_dir(&project)
            .args(["add", "wasm-bindgen"])
            .output()?
            .status;
        assert!(status.success());

        Cargo::new(&project)?.run(&project, false, "release")?;

        let bindgen = WasmBindgen::new(WasmBindgen::find_version(project.join("Cargo.lock"))?)?;
        if !bindgen.installed() {
            bindgen.install()?;
        }

        bindgen.run(
            &project.join("target/wazzup/wasm32-unknown-unknown/debug/temp.wasm"),
            &project.join("dist"),
        )?;

        WasmOpt::run(&project.join("dist/temp_bg.wasm"))?;

        Ok(())
    }
}

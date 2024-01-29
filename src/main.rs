use std::{
    fmt::{self, Display},
    fs,
    io::ErrorKind,
    path::Path,
    thread,
    time::Duration,
};

use color_eyre::{
    eyre::{Result, WrapErr},
    Help,
};
use flume::Selector;
use serde::Deserialize;
use tracing::{debug, error, info, Level};
use tracing_subscriber::{filter::Targets, prelude::*};

use self::{
    cli::{BuildArgs, Command, DevArgs},
    watch::ChangeType,
};
use crate::{
    cli::Cli,
    tools::{Cargo, Rustup, Sass, Tailwind},
};

mod build;
mod cli;
mod minify;
mod server;
mod status;
mod tools;
mod watch;

fn main() -> Result<()> {
    let args = Cli::parse();

    color_eyre::install()?;
    init_logger(args.quiet, args.verbose);

    if !Rustup::check_wasm_target()? {
        Rustup::install_wasm_target()?;
    }

    match args.cmd {
        Command::Status => status::status(&std::env::current_dir()?),
        Command::Build(args) => build(args, false),
        Command::Dev(args) => dev(args),
        Command::Completions { shell } => cli::completions(shell),
        Command::Manpages { dir } => cli::manpages(&dir),
    }
}

/// Initialize the application logger.
fn init_logger(quiet: bool, verbose: u8) {
    if quiet {
        return;
    }

    tracing_subscriber::registry()
        .with(Targets::new().with_default(Level::WARN).with_target(
            env!("CARGO_CRATE_NAME"),
            match verbose {
                0 => Level::INFO,
                1 => Level::DEBUG,
                _ => Level::TRACE,
            },
        ))
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();
}

/// Retrieve the package name from the project's `Cargo.toml`.
fn package_name(project: &Path) -> Result<String> {
    #[derive(Deserialize)]
    struct CargoToml {
        package: Package,
    }

    #[derive(Deserialize)]
    struct Package {
        name: String,
    }

    let buf = fs::read_to_string(project.join("Cargo.toml"))
        .wrap_err("failed to read the Cargo.toml manifest")
        .note("wazzup expects to be run from within a Rust Cargo project")
        .note("ensure you run from the root of the project")?;

    let deser = toml::Deserializer::new(&buf);

    serde_path_to_error::deserialize::<_, CargoToml>(deser)
        .wrap_err("failed to parse the Cargo.toml manifest")
        .map(|toml| toml.package.name)
}

/// The CSS framework that is used by the project. This decides what tools are run when building
/// all components of the project.
#[derive(Clone, Copy, Eq, PartialEq)]
enum CssMode {
    /// The [SASS/SCSS](https://sass-lang.com) framework.
    Sass,
    /// The [TailwindCSS](https://tailwindcss.com) framework.
    Tailwind,
}

impl Display for CssMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CssMode::Sass => "sass",
            CssMode::Tailwind => "tailwind",
        })
    }
}

// Detect which CSS framework is used.
fn css_mode(project: &Path) -> Result<CssMode> {
    match fs::metadata(project.join("tailwind.config.js")) {
        Ok(_) => Ok(CssMode::Tailwind),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(CssMode::Sass),
        Err(e) => Err(e.into()),
    }
}

/// Fully build the project from scratch.
fn build(args: BuildArgs, dev: bool) -> Result<()> {
    let project = std::env::current_dir()?;
    let out = project.join("dist");

    if out.exists() {
        fs::remove_dir_all(&out)
            .wrap_err("failed clearing output directory")
            .with_note(|| format!("output directory: {}", out.display()))
            .suggestion("try deleting the directory manually")?;
    }

    fs::create_dir(&out)
        .wrap_err("failed creating the output directory")
        .with_note(|| format!("output directory: {}", out.display()))?;

    let cargo = Cargo::new(&project)?;
    let name = package_name(&project)?;
    let css_mode = css_mode(&project)?;

    build::index(&project, &name, args.release, &args.base_url, dev)?;
    info!("built index.html");

    match css_mode {
        CssMode::Sass => {
            let sass = Sass::new(cargo.workspace_dir(), &project)?;
            build::sass(&sass, &project, args.release)?
        }
        CssMode::Tailwind => {
            let tailwind = Tailwind::new(cargo.workspace_dir(), &project)?;
            build::tailwind(&tailwind, &project, args.release)?
        }
    }
    info!(mode = %css_mode, "built stylesheets");

    build::assets(&project)?;
    info!("built assets");

    build::rust(&cargo, &project, &name, args.release, &args.profile)?;
    info!("built WASM files");

    if args.release {
        let reduction = minify::html(&project)?;
        info!(%reduction, "minified HTML files");
        let reduction = minify::js(&project)?;
        info!(%reduction, "minified JavaScript files");
        let reduction = minify::wasm(&project)?;
        info!(%reduction, "minified WASM files");
    }

    Ok(())
}

/// Run a local dev server that hosts the project and rebuilds on file changes.
fn dev(args: DevArgs) -> Result<()> {
    let project = std::env::current_dir()?;
    let name = package_name(&project)?;
    let css_mode = css_mode(&project)?;

    let watcher = watch::watch(project.clone())?;
    let debouncer = watch::debounce(watcher, Duration::from_secs(2))?;
    let (shutdown_tx, shutdown_rx) = flume::bounded(0);
    let (reload_tx, reload_rx) = flume::bounded(0);

    let thread = thread::spawn({
        let project = project.clone();
        let cargo = Cargo::new(&project)?;
        let sass = Sass::new(cargo.workspace_dir(), &project)?;
        let tailwind = Tailwind::new(cargo.workspace_dir(), &project)?;

        move || {
            if let Err(e) = build(BuildArgs::default(), true) {
                error!(error = ?e, "failed building");
                return;
            }

            reload_tx.send(()).ok();
            debug!("sent reload signal");

            loop {
                let res = Selector::new()
                    .recv(&shutdown_rx, |_| None)
                    .recv(debouncer.receiver(), |change| change.ok())
                    .wait();

                if let Some(change) = res {
                    if let Err(e) =
                        rebuild(&cargo, &sass, &tailwind, &project, &name, css_mode, change)
                    {
                        error!(error = ?e, "failed rebuilding");
                        continue;
                    }

                    reload_tx.send(()).ok();
                    debug!("sent reload signal");
                } else {
                    debouncer.shutdown().shutdown();
                    break;
                }
            }
        }
    });

    let res = server::run(project, args.port, reload_rx);

    shutdown_tx.send(()).ok();
    thread.join().expect("thread to shut down properly");

    res
}

/// Rebuild parts of the application, based on the kind of source files that changed. For example,
/// only rebuild the WASM binary if Rust code changed or only the stylesheets if any sass/scss/css
/// files changed.
fn rebuild(
    cargo: &Cargo,
    sass: &Sass,
    tailwind: &Tailwind,
    project: &Path,
    name: &str,
    css_mode: CssMode,
    change: ChangeType,
) -> Result<()> {
    // Tailwind scans project files to detect what CSS classes are used. Therefore, we have to run
    // it not just when CSS files changed, but when HTML or Rust files changed as well.
    if css_mode == CssMode::Tailwind
        && matches!(
            change,
            ChangeType::Html | ChangeType::Css | ChangeType::Rust
        )
    {
        build::tailwind(tailwind, project, false)?;
        info!(mode = %css_mode, "rebuilt stylesheets");
    }

    match change {
        ChangeType::Html => {
            build::index(project, name, false, "/", true)?;
            info!("rebuilt index.html");
        }
        ChangeType::Css => {
            if css_mode == CssMode::Sass {
                build::sass(sass, project, false)?;
                info!(mode = %css_mode, "rebuilt stylesheets");
            }
        }
        ChangeType::Static(asset) => {
            build::asset(project, asset.strip_prefix(project)?)?;
            info!("rebuilt asset");
        }
        ChangeType::Rust => {
            build::rust(cargo, project, name, false, "release")?;
            info!("rebuilt WASM files");
        }
    }

    Ok(())
}

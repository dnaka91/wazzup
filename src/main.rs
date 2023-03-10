use std::{fs, path::Path, thread, time::Duration};

use crate::{cli::Cli, tools::Rustup};
use anyhow::Result;
use flume::Selector;
use serde::Deserialize;
use tracing::{error, info, Level};
use tracing_subscriber::{filter::Targets, prelude::*};

use self::{
    cli::{BuildArgs, Command, DevArgs},
    watch::ChangeType,
};

mod build;
mod cli;
mod server;
mod status;
mod tools;
mod watch;

fn main() -> Result<()> {
    let args = Cli::parse();

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

    let buf = fs::read_to_string(project.join("Cargo.toml"))?;

    toml::from_str::<CargoToml>(&buf)
        .map_err(Into::into)
        .map(|toml| toml.package.name)
}

/// Fully build the project from scratch.
fn build(args: BuildArgs, dev: bool) -> Result<()> {
    let project = std::env::current_dir()?;
    let out = project.join("dist");

    if out.exists() {
        fs::remove_dir_all(&out)?;
    }

    fs::create_dir(&out)?;

    let name = package_name(&project)?;

    build::index(&project, &name, args.release, dev)?;
    info!("built index.html");
    build::rust(&project, &name, args.release, &args.profile)?;
    info!("built WASM files");
    build::sass(&project, args.release)?;
    info!("built stylesheets");
    build::assets(&project)?;
    info!("built assets");

    Ok(())
}

/// Run a local dev server that hosts the project and rebuilds on file changes.
fn dev(args: DevArgs) -> Result<()> {
    build(BuildArgs::default(), true)?;

    let project = std::env::current_dir()?;
    let name = package_name(&project)?;

    let watcher = watch::watch(project.clone())?;
    let debouncer = watch::debounce(watcher, Duration::from_secs(2))?;
    let (shutdown_tx, shutdown_rx) = flume::bounded(0);
    let (rebuild_tx, rebuild_rx) = flume::bounded(0);

    let thread = thread::spawn({
        let project = project.clone();

        move || loop {
            let res = Selector::new()
                .recv(&shutdown_rx, |_| None)
                .recv(debouncer.receiver(), |change| change.ok())
                .wait();

            if let Some(change) = res {
                if let Err(e) = rebuild(&project, &name, change) {
                    error!(error = %e, "failed rebuilding");
                    continue;
                }

                rebuild_tx.send(()).ok();
            } else {
                debouncer.shutdown().shutdown();
                break;
            }
        }
    });

    let res = server::run(project, args.port, rebuild_rx);

    shutdown_tx.send(()).ok();
    thread.join().expect("thread to shut down properly");

    res
}

/// Rebuild parts of the application, based on the kind of source files that changed. For example,
/// only rebuild the WASM binary if Rust code changed or only the stylesheets if any sass/scss/css
/// files changed.
fn rebuild(project: &Path, name: &str, change: ChangeType) -> Result<()> {
    match change {
        ChangeType::Html => {
            build::index(project, name, false, true)?;
            info!("rebuilt index.html");
        }
        ChangeType::Sass => {
            build::sass(project, false)?;
            info!("rebuilt stylesheets");
        }
        ChangeType::Static(asset) => {
            build::asset(project, asset.strip_prefix(project)?)?;
            info!("rebuilt asset");
        }
        ChangeType::Rust => {
            build::rust(project, name, false, "release")?;
            info!("rebuilt WASM files");
        }
    }

    Ok(())
}

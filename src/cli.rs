//! Data structures for command line arguments parsing logic for them.

use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::{Path, PathBuf},
};

use anyhow::{ensure, Context, Result};
use clap::{ArgAction, Args, CommandFactory, Parser, Subcommand, ValueHint};
use clap_complete::Shell;

#[derive(Debug, Parser)]
#[command(about, author, version)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Cli {
    /// Only show minimal output. Mutually exclusive with the --verbose flag
    #[arg(long, short, global = true, conflicts_with = "verbose")]
    pub quiet: bool,
    /// Increase the verbosity of status messages. This argument can be set multiple times to
    /// further raise the verbosity level.
    #[arg(long, short, global = true, action = ArgAction::Count)]
    pub verbose: u8,
    #[command(subcommand)]
    pub cmd: Command,
}

impl Cli {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}

#[derive(Debug, Subcommand)]
#[cfg_attr(test, derive(PartialEq))]
pub enum Command {
    /// Show the status of various needed components and current project setup.
    Status,
    /// Build the project.
    Build(BuildArgs),
    /// Run a local server for development purposes.
    Dev(DevArgs),
    /// Generate auto-completion scripts for various shells.
    Completions {
        /// Shell to generate an auto-completion script for.
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Generate man pages into the given directory.
    Manpages {
        /// Target directory, that must already exist and be empty. If the any file with the same
        /// name as any of the man pages already exist, it'll not be overwritten, but instead an
        /// error be returned.
        #[arg(value_hint = ValueHint::DirPath)]
        dir: PathBuf,
    },
}

#[derive(Debug, Args)]
#[cfg_attr(test, derive(PartialEq))]
pub struct BuildArgs {
    /// Build in release mode.
    #[arg(long, short)]
    pub release: bool,
    /// The actual profile to use for release mode.
    #[arg(long, default_value = "release")]
    pub profile: String,
}

impl Default for BuildArgs {
    fn default() -> Self {
        Self {
            release: false,
            profile: "release".to_owned(),
        }
    }
}

#[derive(Debug, Args)]
#[cfg_attr(test, derive(PartialEq))]
pub struct DevArgs {
    /// The local TCP port to listen on.
    #[arg(long, short, default_value_t = 8080)]
    pub port: u16,
}

impl Default for DevArgs {
    fn default() -> Self {
        Self { port: 8080 }
    }
}

/// Generate shell completions, written to the standard output.
pub fn completions(shell: Shell) -> Result<()> {
    clap_complete::generate(
        shell,
        &mut Cli::command(),
        env!("CARGO_PKG_NAME"),
        &mut io::stdout().lock(),
    );
    Ok(())
}

/// Generate man pages in the target directory. The directory must already exist and none of the
/// files exist, or an error is returned.
pub fn manpages(dir: &Path) -> Result<()> {
    fn print(dir: &Path, app: &clap::Command) -> Result<()> {
        let name = app.get_display_name().unwrap_or_else(|| app.get_name());
        let out = dir.join(format!("{name}.1"));
        let mut out = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&out)
            .with_context(|| format!("the file `{}` already exists", out.display()))?;

        clap_mangen::Man::new(app.clone()).render(&mut out)?;
        out.flush()?;

        for sub in app.get_subcommands() {
            print(dir, sub)?;
        }

        Ok(())
    }

    ensure!(dir.try_exists()?, "target directory doesn't exist");

    let mut app = Cli::command();
    app.build();

    print(dir, &app)
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    #[test]
    fn default_build_args() {
        let cli = Cli::parse_from(["app", "build"]);
        let expect = Cli {
            quiet: false,
            verbose: 0,
            cmd: Command::Build(BuildArgs::default()),
        };

        assert_eq!(expect, cli);
    }

    #[test]
    fn default_dev_args() {
        let cli = Cli::parse_from(["app", "dev"]);
        let expect = Cli {
            quiet: false,
            verbose: 0,
            cmd: Command::Dev(DevArgs::default()),
        };

        assert_eq!(expect, cli);
    }
}

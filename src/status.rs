//! Status report for required external tools and mandatory project files.

use std::{
    fmt::{self, Display},
    path::{Path, PathBuf},
};

use color_eyre::Result;
use tabled::{
    settings::{object::Rows, style::HorizontalLine, Alignment, Modify, Panel, Style},
    Table, Tabled,
};
use yansi::{Color, Paint};

use crate::tools::{self, Cargo};

/// Display the status of required external tools, and mandatory files within the current project.
pub fn status(project: &Path) -> Result<()> {
    let cargo = Cargo::new(project)?;
    let root = cargo.workspace_dir();

    print_table(
        "Tools",
        [
            tool_status_sys("rustup")?,
            tool_status_sys("cargo")?,
            tool_status_sys("wasm-opt")?,
            tool_status_js("sass", root, project)?,
            tool_status_js("tailwindcss", root, project)?,
        ],
    );

    print_table(
        "Project files",
        [
            project_file_status(
                project,
                ["assets/main.sass", "assets/main.scss", "assets/main.css"],
            ),
            project_file_status(project, [".gitignore"]),
            project_file_status(project, ["Cargo.lock"]),
            project_file_status(project, ["Cargo.toml"]),
            project_file_status(project, ["index.html"]),
        ],
    );

    Ok(())
}

/// Print out the table data, with an additional header above it, and nice border formatting.
fn print_table(header: &str, values: impl IntoIterator<Item = impl Tabled>) {
    let table = Table::new(values)
        .with(Style::modern().remove_horizontal().horizontals([
            (0, HorizontalLine::full('─', '─', '┌', '┐')),
            (1, HorizontalLine::full('─', '┬', '├', '┤')),
            (2, HorizontalLine::full('─', '┼', '├', '┤')),
        ]))
        .with(Panel::header(Paint::new(header).bold().to_string()))
        .with(Modify::new(Rows::first()).with(Alignment::center()))
        .to_string();

    println!("{table}");
}

/// Information about a single external tool.
#[derive(Tabled)]
struct Tool {
    /// Tool name, which is the binary's name as well.
    name: &'static str,
    /// Availability of the tool.
    status: FileStatus,
    /// Absolute path to the tool for invocation.
    #[tabled(display_with = "display_pathbuf_opt")]
    path: Option<PathBuf>,
}

/// Information about a single project file.
#[derive(Tabled)]
struct ProjectFile {
    /// Path of the file, relative to the project root.
    #[tabled(display_with = "display_pathbuf")]
    path: PathBuf,
    /// Availability of the file.
    status: FileStatus,
}

/// Status of a file.
enum FileStatus {
    /// File was found.
    Found,
    /// File is missing.
    Missing,
}

impl Display for FileStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Found => Paint::new("found").fg(Color::Green),
                Self::Missing => Paint::new("missing").fg(Color::Red),
            }
        )
    }
}

/// Helper for [`tabled`], to display a [`Path`].
fn display_pathbuf(v: &Path) -> String {
    v.display().to_string()
}

/// Helper for [`tabled`], to display an [`Option`]<[`PathBuf`]>.
#[expect(clippy::ref_option)]
fn display_pathbuf_opt(v: &Option<PathBuf>) -> String {
    match v {
        Some(path) => path.display().to_string(),
        None => String::new(),
    }
}

/// Determine the installation status of an external, system-installed tool.
fn tool_status_sys(name: &'static str) -> Result<Tool> {
    tool_status(name, tools::find_bin)
}

fn tool_status_js(name: &'static str, root: &Path, cwd: &Path) -> Result<Tool> {
    tool_status(name, |name| tools::find_bin_js(name, root, cwd))
}

fn tool_status(name: &'static str, find: impl Fn(&'static str) -> Result<PathBuf>) -> Result<Tool> {
    let (path, status) = match find(name) {
        Ok(path) => (Some(path), FileStatus::Found),
        Err(report) => match report.downcast_ref::<which::Error>() {
            Some(which::Error::CannotFindBinaryPath) => (None, FileStatus::Missing),
            _ => return Err(report),
        },
    };

    Ok(Tool { name, status, path })
}

/// Determine the status of a file within the current project.
fn project_file_status<const N: usize>(project: &Path, paths: [&'static str; N]) -> ProjectFile {
    let (path, status) = paths
        .iter()
        .find_map(|path| {
            let full_path = project.join(path);
            full_path.exists().then(|| PathBuf::from(path))
        })
        .map_or_else(
            || {
                (
                    paths.first().map(PathBuf::from).unwrap_or_default(),
                    FileStatus::Missing,
                )
            },
            |path| (path, FileStatus::Found),
        );

    ProjectFile { path, status }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_status() -> Result<()> {
        status(&std::env::current_dir()?.join("sample"))?;
        Ok(())
    }
}

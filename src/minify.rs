use std::{
    fmt::{self, Display},
    fs::{self, Metadata},
    path::Path,
};

use anyhow::{anyhow, Result};
use ignore::{DirEntry, WalkBuilder};

use crate::tools::WasmOpt;

#[derive(Default)]
pub struct Reduction {
    original: usize,
    minified: usize,
}

impl Display for Reduction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ratio = if self.original == 0 {
            0.0
        } else {
            self.minified as f64 / self.original as f64
        };

        write!(f, "{:.1}%", (1.0 - ratio) * 100.0)
    }
}

pub fn html(project: &Path) -> Result<Reduction> {
    let cfg = minify_html::Cfg {
        do_not_minify_doctype: true,
        minify_css: true,
        minify_js: true,
        ..minify_html::Cfg::default()
    };

    let mut reduction = Reduction::default();

    for file in find_files(project.join("dist"), "html") {
        let (entry, _) = file?;

        let original = fs::read(entry.path())?;
        let minified = minify_html::minify(&original, &cfg);

        reduction.original += original.len();
        reduction.minified += minified.len();

        fs::write(entry.into_path(), minified)?;
    }

    Ok(reduction)
}

pub fn js(project: &Path) -> Result<Reduction> {
    let mut reduction = Reduction::default();
    let mut session = minify_js::Session::new();

    for file in find_files(project.join("dist"), "js") {
        let (entry, _) = file?;

        let original = fs::read(entry.path())?;
        let mut minified = Vec::with_capacity(original.len());

        reduction.original += original.len();

        session.reset();
        minify_js::minify(
            &session,
            minify_js::TopLevelMode::Module,
            &original,
            &mut minified,
        )
        .map_err(|e| anyhow!("failed minifying JavaScript: {e}"))?;

        reduction.minified += minified.len();

        fs::write(entry.into_path(), minified)?;
    }

    Ok(reduction)
}

pub fn wasm(project: &Path) -> Result<Reduction> {
    let mut reduction = Reduction::default();

    for file in find_files(project.join("dist"), "wasm") {
        let (entry, metadata) = file?;

        reduction.original += metadata.len() as usize;

        WasmOpt::run(entry.path())?;

        reduction.minified += entry.metadata()?.len() as usize;
    }

    Ok(reduction)
}

fn find_files(
    root: impl AsRef<Path>,
    extension: &'_ str,
) -> impl Iterator<Item = Result<(DirEntry, Metadata)>> + '_ {
    WalkBuilder::new(root)
        .standard_filters(false)
        .require_git(false)
        .git_ignore(true)
        .parents(true)
        .build()
        .filter_map(move |entry| {
            let info = || {
                let entry = entry?;
                let metadata = entry.metadata()?;

                if metadata.is_dir() || entry.path().extension().unwrap_or_default() != extension {
                    return Ok(None);
                }

                Ok(Some((entry, metadata)))
            };

            info().transpose()
        })
}

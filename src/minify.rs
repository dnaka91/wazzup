use std::{
    fmt::{self, Display},
    path::Path,
};

use anyhow::{anyhow, Result};
use ignore::WalkBuilder;

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
    let walk = WalkBuilder::new(project.join("dist"))
        .standard_filters(false)
        .build();

    let cfg = minify_html::Cfg {
        do_not_minify_doctype: true,
        minify_css: true,
        minify_js: true,
        ..minify_html::Cfg::default()
    };

    let mut reduction = Reduction::default();

    for entry in walk {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_dir() || entry.path().extension().unwrap_or_default() != "html" {
            continue;
        }

        let original = std::fs::read(entry.path())?;
        let minified = minify_html::minify(&original, &cfg);

        reduction.original += original.len();
        reduction.minified += minified.len();

        std::fs::write(entry.into_path(), minified)?;
    }

    Ok(reduction)
}

pub fn js(project: &Path) -> Result<Reduction> {
    let walk = WalkBuilder::new(project.join("dist"))
        .standard_filters(false)
        .build();

    let mut reduction = Reduction::default();

    for entry in walk {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_dir() || entry.path().extension().unwrap_or_default() != "js" {
            continue;
        }

        let original = std::fs::read(entry.path())?;
        let mut minified = Vec::with_capacity(original.len());

        reduction.original += original.len();

        minify_js::minify(minify_js::TopLevelMode::Module, original, &mut minified)
            .map_err(|e| anyhow!("failed minifying JavaScript: {e}"))?;

        reduction.minified += minified.len();

        std::fs::write(entry.into_path(), minified)?;
    }

    Ok(reduction)
}

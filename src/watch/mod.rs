//! Logic for watching a project's source files and assets to determine what part of the
//! application should be rebuilt.
//!
//! Files are categories as follows, and sent as events as the [`ChangeType`] type.
//! - HTML
//!   - index.html
//! - SASS/SCSS/CSS
//!   - assets/main.{sass,scss,css}
//!   - assets/{sass,scss,css}
//! - Static files
//!   - assets/* (excluding SASS/SCSS/CSS files)
//! - Rust
//!   - Any remaining files, due to the fact that other files might be:
//!     - Included with include_bytes!, include_str! or other macros
//!     - Processed through proc macros and turned into Rust code (for example Protobuf schemas)

mod debouncer;
mod watcher;

use std::{
    fmt::{self, Display},
    path::PathBuf,
};

pub use debouncer::debounce;
pub use watcher::watch;

/// Size for any message channels used within the watcher and debouncer.
const CHANNEL_SIZE: usize = 16;

/// Identifier for what part of the project was changed in the file system.
///
/// This can then be used, to identify which part to rebuild, instead of building the whole project
/// all over again.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ChangeType {
    /// The `index.html` file.
    Html,
    /// Main SASS/SCSS/CSS file or any file referenced by it.
    Css,
    /// Static files that are not SASS/SCSS/CSS files. Therefore, copied as-is without any
    /// processing.
    Static(PathBuf),
    /// Rust source files, or any other remaining file, as it could be included by a Rust file.
    Rust,
}

impl Display for ChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Html => f.write_str("html"),
            Self::Css => f.write_str("css"),
            Self::Static(path) => write!(f, "static:{}", path.display()),
            Self::Rust => f.write_str("rust"),
        }
    }
}

use std::{
    fs::{self, File},
    io::{BufWriter, ErrorKind, Write},
    path::Path,
};

use color_eyre::{
    eyre::{eyre, Context, Result},
    Help, SectionExt,
};
use ignore::WalkBuilder;

use crate::tools::{Cargo, Sass, Tailwind, WasmBindgen};

pub fn index(
    project: &Path,
    app_name: &str,
    release: bool,
    base_url: &str,
    dev: bool,
) -> Result<()> {
    let index = load_index(project)?;
    transform_index(&index, app_name, project, release, base_url, dev)
}

struct HtmlIndex {
    top: String,
    middle: String,
    bottom: String,
}

fn load_index(project: &Path) -> Result<HtmlIndex> {
    const TRIM_CHARS: &[char] = &[' ', '\t'];

    let path = project.join("index.html");
    let data = fs::read_to_string(path)
        .wrap_err("failed to read the project's `index.html`")
        .note("your project must contain a `index.html` file at its root")
        .suggestion("check the status with `wazzup status`")?;

    let (top, rest) = data
        .split_once("<!--WAZZUP-HEAD-->")
        .ok_or_else(|| eyre!("missing WAZZUP-HEAD marker"))
        .note("the index.html must contain a <!--WAZZUP-HEAD--> comment in the head section")
        .with_section(|| data.trim().to_owned().header("index.html:"))?;
    let (middle, bottom) = rest
        .split_once("<!--WAZZUP-BODY-->")
        .ok_or_else(|| eyre!("missing WAZZUP-BODY marker"))
        .note("the index.html must contain a <!--WAZZUP-BODY--> comment in the body section")
        .with_section(|| data.trim().to_owned().header("index.html:"))?;

    Ok(HtmlIndex {
        top: top.trim_matches(TRIM_CHARS).to_owned(),
        middle: middle.trim_end_matches(TRIM_CHARS).to_owned(),
        bottom: bottom.trim_matches(TRIM_CHARS).to_owned(),
    })
}

fn transform_index(
    index: &HtmlIndex,
    app_name: &str,
    project: &Path,
    release: bool,
    base_url: &str,
    dev: bool,
) -> Result<()> {
    let mut file = BufWriter::new(File::create(project.join("dist/index.html"))?);
    let base = base_url.strip_suffix('/').unwrap_or(base_url);

    file.write_all(index.top.as_bytes())?;

    if !release {
        writeln!(&mut file, r#"    <!-- stylesheet -->"#)?;
    }
    write!(
        &mut file,
        r#"    <link rel="stylesheet" href="{base}/main.css">"#
    )?;

    file.write_all(index.middle.as_bytes())?;

    if !release {
        writeln!(&mut file, r#"    <!-- WASM initialization -->"#)?;
    }
    writeln!(&mut file, r#"    <script type="module">"#)?;
    writeln!(
        &mut file,
        r#"      import init from '{base}/{app_name}.js';"#,
    )?;
    writeln!(
        &mut file,
        r#"      await init('{base}/{app_name}_bg.wasm');"#,
    )?;
    write!(&mut file, r#"    </script>"#)?;

    if dev {
        writeln!(&mut file)?;
        writeln!(&mut file, r#"    <!-- dev page reload script -->"#)?;
        write!(
            &mut file,
            r#"    <script src="/__WAZZUP__/reload.js"></script>"#
        )?;
    }

    file.write_all(index.bottom.as_bytes())?;
    file.into_inner()?.flush()?;

    Ok(())
}

pub fn rust(
    cargo: &Cargo,
    project: &Path,
    app_name: &str,
    release: bool,
    profile: &str,
) -> Result<()> {
    cargo.run(project, release, profile)?;

    let bindgen = WasmBindgen::new(WasmBindgen::find_version(
        cargo.workspace_dir().join("Cargo.lock"),
    )?)?;
    if !bindgen.installed() {
        bindgen.install()?;
    }

    bindgen.run(
        &cargo.target_dir().join(format!(
            "wazzup/wasm32-unknown-unknown/{profile}/{app_name}.wasm",
            profile = if release { profile } else { "debug" },
        )),
        &project.join("dist"),
    )?;

    Ok(())
}

pub fn sass(sass: &Sass, project: &Path, release: bool) -> Result<()> {
    let stylesheets = [
        project.join("assets/main.sass"),
        project.join("assets/main.scss"),
        project.join("assets/main.css"),
    ];

    if let Some(stylesheet) = stylesheets.iter().find(|path| path.exists()) {
        sass.run(stylesheet, &project.join("dist/main.css"), release)?;
    }

    Ok(())
}

pub fn tailwind(tailwind: &Tailwind, project: &Path, release: bool) -> Result<()> {
    tailwind.run(
        &project.join("assets/main.css"),
        &project.join("dist/main.css"),
        release,
    )
}

pub fn assets(project: &Path) -> Result<()> {
    let assets = project.join("assets");

    let walk = WalkBuilder::new(&assets)
        .standard_filters(false)
        .require_git(false)
        .git_ignore(true)
        .parents(true)
        .filter_entry(move |entry| {
            let Ok(path) = entry.path().strip_prefix(&assets) else {
                return false;
            };
            path != Path::new("main.sass")
                && path != Path::new("main.scss")
                && path != Path::new("main.css")
                && !path.starts_with("sass/")
                && !path.starts_with("scss/")
                && !path.starts_with("css/")
        })
        .build();

    let assets = project.join("assets");
    let dist = project.join("dist");

    for entry in walk.skip(1) {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            continue;
        }

        let source_path = entry.path();
        let target_path = dist.join(source_path.strip_prefix(&assets)?);

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::copy(source_path, target_path)?;
    }

    Ok(())
}

pub fn asset(project: &Path, asset: &Path) -> Result<()> {
    let full_path = project.join(asset);
    let dist_path = project.join("dist").join(asset.strip_prefix("assets/")?);

    if full_path.exists() {
        let metadata = fs::metadata(&full_path)?;
        if metadata.is_dir() {
            fs::create_dir_all(dist_path)?;
        } else {
            if let Some(parent) = dist_path.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::copy(full_path, dist_path)?;
        }

        Ok(())
    } else {
        match fs::remove_dir_all(dist_path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use assert_fs::prelude::*;
    use color_eyre::Result;
    use indoc::indoc;

    use crate::tools::Sass;

    const INDEX_HTML: &str = indoc! {r"
        <!DOCTYPE html>
        <html>
          <head>
            <!--WAZZUP-HEAD-->
          </head>
          <body>
            <!--WAZZUP-BODY-->
          </body>
        </html>
    "};

    #[test]
    fn build_index_debug() -> Result<()> {
        let temp = assert_fs::TempDir::new()?;

        temp.child("index.html").write_str(INDEX_HTML)?;
        temp.child("dist").create_dir_all()?;

        super::index(temp.path(), "test", false, "/", false)?;

        temp.child("dist/index.html").assert(indoc! {r#"
                <!DOCTYPE html>
                <html>
                  <head>
                    <!-- stylesheet -->
                    <link rel="stylesheet" href="/main.css">
                  </head>
                  <body>
                    <!-- WASM initialization -->
                    <script type="module">
                      import init from '/test.js';
                      await init('/test_bg.wasm');
                    </script>
                  </body>
                </html>
            "#});

        Ok(())
    }

    #[test]
    fn build_index_debug_dev() -> Result<()> {
        let temp = assert_fs::TempDir::new()?;

        temp.child("index.html").write_str(INDEX_HTML)?;
        temp.child("dist").create_dir_all()?;

        super::index(temp.path(), "test", false, "/", true)?;

        temp.child("dist/index.html").assert(indoc! {r#"
                <!DOCTYPE html>
                <html>
                  <head>
                    <!-- stylesheet -->
                    <link rel="stylesheet" href="/main.css">
                  </head>
                  <body>
                    <!-- WASM initialization -->
                    <script type="module">
                      import init from '/test.js';
                      await init('/test_bg.wasm');
                    </script>
                    <!-- dev page reload script -->
                    <script src="/__WAZZUP__/reload.js"></script>
                  </body>
                </html>
            "#});

        Ok(())
    }

    #[test]
    fn build_index_release() -> Result<()> {
        let temp = assert_fs::TempDir::new()?;

        temp.child("index.html").write_str(INDEX_HTML)?;
        temp.child("dist").create_dir_all()?;

        super::index(temp.path(), "test", true, "/", false)?;

        temp.child("dist/index.html").assert(indoc! {r#"
                <!DOCTYPE html>
                <html>
                  <head>
                    <link rel="stylesheet" href="/main.css">
                  </head>
                  <body>
                    <script type="module">
                      import init from '/test.js';
                      await init('/test_bg.wasm');
                    </script>
                  </body>
                </html>
            "#});

        Ok(())
    }

    #[test]
    fn build_sass() -> Result<()> {
        let temp = assert_fs::TempDir::new()?;
        temp.child("assets/main.sass").write_str(indoc! {"
            .test
              font-size: 16pt
        "})?;

        let sass = Sass::new(temp.path(), temp.path())?;
        super::sass(&sass, temp.path(), true)?;

        temp.child("dist/main.css")
            .assert(".test{font-size:16pt}\n");

        Ok(())
    }

    #[test]
    fn build_assets() -> Result<()> {
        let temp = assert_fs::TempDir::new()?;
        temp.child("assets/test1.txt").write_str("test1")?;
        temp.child("assets/t2/test2.txt").write_str("test2")?;

        super::assets(temp.path())?;

        temp.child("dist/test1.txt").assert("test1");
        temp.child("dist/t2/test2.txt").assert("test2");

        Ok(())
    }

    #[test]
    fn build_asset() -> Result<()> {
        let temp = assert_fs::TempDir::new()?;
        temp.child("assets/t/t/t/test.txt").write_str("test")?;

        super::asset(temp.path(), Path::new("assets/t/t/t/test.txt"))?;

        temp.child("assets/t/t/t/test.txt").assert("test");

        Ok(())
    }
}

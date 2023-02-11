# 🍹 WazzUp

My approach on a very opinionated Rust WASM project builder.

## Use case

This project is meant for one specific purpose: Build WASM projects fully in Rust, that are supposed to run either in the browser or with [Tauri](https://tauri.app/). Furthermore, it expects a specific project layout and is exceptionally limited on configuration options.

That means no mix of JS and Rust WASM, no building as a library or any of those combinations. Full blown Rust projects and nothing else.

Probably worth noting, this is my personal project for my specific setup and layouts and what not. Meaning, it's unlikely any request for new features will be accepted. If it matches your use case, great. If it doesn't, well... fork it 😛.

## Prerequisites

The following tools must be pre-installed on your system, and available through your `$PATH` variable:

- Rust installed through `rustup`.
- Binaryen toolset, which includes `wasm-opt`.
- Dart-sass compiler.

This project relies on `wasm-bindgen` as well, but the version is strictly bound to the one used in your projects. Therefore, it'll detect the right version and install it through `cargo` into its own cache folder. That means it won't clash with any version that you might have already installed with `cargo` or a package manager yourself.

### I use Arch btw

Fellow Arch users can simply installed the needed dependencies as follows:

```sh
sudo pacman -S --needed binaryen dart-sass rustup
```

## Installation

This project is currently not published on crates.io, and I don't feel the need to provide pre-compiled binaries. Instead, just use cargo:

```sh
cargo install --git https://github.com/dnaka91/wazzup.git
```

## Layout

To keep configuration options to a minimum, a project must adhere to a certain file structure and
at least contain the following files:

- `assets/main.sass`: Single source for any styling. Can **alternatively** be `main.scss` or `main.css`. Must only reference files from the `assets/{sass,scss,css}/` folder.
- `assets/sass/`: Additional dependencies for the `main.{sass,scss,css}` file. The `assets/scss/` and `assets/css/` folders can coexist next to it, if needed.
- `assets/*`: Remaining assets, that are not stylesheets.
- `src/`: All the Rust code.
- `Cargo.lock`: Lock file for Rust dependencies, and mandatory to detect the used `wasm-bindgen` version.
- `Cargo.toml`: Typical Rust project config file.
- `index.html`: Main HTML file.
- `.gitignore`: Ignore patterns for files in the repository, that won't be watched by this project for file change detection.

A minimal project would look like this:

```txt
.
├── assets
│  └── main.sass
├── src
│  └── main.rs
├── .gitignore
├── Cargo.lock
├── Cargo.toml
└── index.html
```

Furthermore, the output is assembled in the `dist` folder, including the WASM binary, JS glue, stylesheets, index page and any additional assets. That means, the `.gitignore` should at least include:

```sh
dist/
target/
```

## Usage

The application currently has **three** main commands: `status`, `build` and `dev`.

- `status` searches for all needed external programs and mandatory project files, reporting there status in nice ASCII tables.
- `build` compiles the project and all its assets into the `dist/` directory.
- `dev` spins up a local development server and rebuilds the project on file changes.

For further details, simply run the application with the `-h`/`--help` flag. The usage should be pretty self-explanatory from there on.

### Setup with Tauri

To configure Tauri to use Wazzup, adjust your `tauri.conf.json` as follows:

```json
{
  "build": {
    "beforeBuildCommand": "wazzup build --release",
    "beforeDevCommand": "wazzup dev",
    "devPath": "http://localhost:8080",
    "distDir": "../dist"
    // other build config ...
  }
  // other settings ...
}
```

## License

This project is licensed under the [AGPL-3.0 License](LICENSE) (or <https://www.gnu.org/licenses/agpl-3.0.html>).

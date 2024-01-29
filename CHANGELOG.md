# Changelog

All notable changes to this project will be documented in this file.

<!-- markdownlint-disable no-duplicate-header -->
<!-- markdownlint-disable no-trailing-spaces -->


## [0.4.2](https://github.com/dnaka91/wazzup/compare/v0.4.1...v0.4.2) - 2024-01-29

### <!-- 0 -->⛰️ Features

- Add TailwindCSS to the tool status list ([3cc9017](https://github.com/dnaka91/wazzup/commit/3cc9017c3958c726d2bdc84cebaad3a18f29e9d0))
  > As wazzup has support for TailwindCSS for a while now, it should show
  > the installation status in addition to only SASS.
- Several overall improvements and dependency updates ([bb62fad](https://github.com/dnaka91/wazzup/commit/bb62fad9a69ecafe4479b4e59b59aa768f753c73))
  > Small tweaks here and there to improve stability and smaller features
  > collected over time, and dependency updates as well.

### <!-- 2 -->📚 Documentation

- Improve changelog format ([c5bb23b](https://github.com/dnaka91/wazzup/commit/c5bb23bfdcc98756e7b7bc9c4db3ad05fc2290a6))
  > Make the changelog more colorful (inspired by `git-cliff`'s own config),
  > by introducing a few emojis and rendering the commit body if present.

## [0.4.1](https://github.com/dnaka91/wazzup/compare/v0.4.0...v0.4.1) - 2023-08-23

### <!-- 0 -->⛰️ Features

- Enable automatic wrapping of terminal help messages ([76bcd1d](https://github.com/dnaka91/wazzup/commit/76bcd1db3bb99cdae78f846f804cdee7e1bb738e))
  > This allows for nicer looking messages when invoking the -h/--help
  > argument, by auto-wrapping and aligning the messages to the current
  > terminal width.
- Allow passing a custom base URL when building a project ([10d1d07](https://github.com/dnaka91/wazzup/commit/10d1d073ad3115feeeef2b2fdf8d71ad394b8660))
  > When not hosting the project at the root of a domain or subdomain, the
  > paths in the `index.html` page must be adjusted to cope for the
  > different base.
  > 
  > The new argument allows to prefix all paths with a custom base URL as
  > well as making them relative by passing `.` as value.

### <!-- 2 -->📚 Documentation

- Correct spelling error in an error message ([5a20a1c](https://github.com/dnaka91/wazzup/commit/5a20a1c675426743bd77a600df2b81d0724b2ddd))

### <!-- 7 -->⚙️ Miscellaneous Tasks

- Include the tag in pre-compiled release binary names ([00607a9](https://github.com/dnaka91/wazzup/commit/00607a9ebfe689cac450823eafe5fa8e4312b108))

## [0.4.0](https://github.com/dnaka91/wazzup/compare/v0.3.3...v0.4.0) - 2023-08-22

### <!-- 0 -->⛰️ Features

- Reduce crate package size ([2fe5a78](https://github.com/dnaka91/wazzup/commit/2fe5a78d4952e1dd0d7eb5a0dc0f2182bb527b4a))
- Improve error messages with suggestions ([4fc956d](https://github.com/dnaka91/wazzup/commit/4fc956d711ec554a9a74315a64bc4e3728a8d20f))
  > Expand the error messages with several extra notes and suggestion to
  > guide users on how to fix them. Also, print out CLI flags if any of the
  > invoked tools fail.
- Start up the dev server before the build finishes ([cef685a](https://github.com/dnaka91/wazzup/commit/cef685a86982643550769437501b4b10776563e4))
  > When used in combination with Tauri, the `cargo-tauri` program waits for
  > the server to accept connections before it continues its own build
  > process.
  > 
  > By starting up the server first, it can proceed early and build in
  > parallel, making the intial startup faster.
- Log external tool invocation in verbose mode ([388182a](https://github.com/dnaka91/wazzup/commit/388182a62d6636534601f394d2a025c1289d99ae))
  > When running in (very) verbose mode (double -v flag / -vv), the
  > arguments with which each tool is invoked are logged before running
  > them.
- Use project-local tailwind binary if available ([9eea088](https://github.com/dnaka91/wazzup/commit/9eea088ade5d5817c36c0de5e5162666fdaa06ad))
  > When invoking tailwind, the local project's `node_modules` folder is
  > added to the search path, which will prefer it over the globally
  > installed version.
- Add CI for testing and pre-compiled binaries ([809d8fe](https://github.com/dnaka91/wazzup/commit/809d8fe8e9ce262f389f9da50bdeb1400a8b1ad7))
  > Added a common GitHub Actions setup that will test the project itself to
  > ensure code quality and avoid mistakes.
  > 
  > In addition, setup a pipeline that will automatically create release
  > logs on new tags, build pre-compiled binaries and add them to the
  > release.
  > 
  > Tools like `cargo-binstall` will be able to utilize this for quicker
  > installations, especially in CI.

### <!-- 1 -->🐛 Bug Fixes

- Ensure the .git folder is properly excluded ([6a2f09e](https://github.com/dnaka91/wazzup/commit/6a2f09e0def918188b5334af0f5ceb021820401f))
  > The .git folder wasn't properly added to the glob patterns which caused
  > unwanted change detection in this folder. An updated glob pattern now
  > correctly excludes the folder.
- Ensure a path is not ignored when unwatching ([6145067](https://github.com/dnaka91/wazzup/commit/614506718fc7e59161b3268920c7e6cc509ef743))
  > When a file or directory was deleted, it was unconditionally removed
  > from the watch list. Now it's correctly checked against the ignorelist
  > first, to reduce the amount of warning logs.

### <!-- 4 -->🚜 Refactor

- Replace `anyhow` with `color-eyre` for better error reporting ([7e8f3b6](https://github.com/dnaka91/wazzup/commit/7e8f3b66f665dc63c576ee73db66b25a55bf9524))

### <!-- 7 -->⚙️ Miscellaneous Tasks

- Update all dependencies ([28f530b](https://github.com/dnaka91/wazzup/commit/28f530b4baf56d691f4b9224abeb934c8a0d8699))
- Extend crate metadata for crates.io ([3dea44a](https://github.com/dnaka91/wazzup/commit/3dea44a988ca23db68e5fee7bf140d9e4729c6a1))
- Reduce the CI setup to only pre-compiled binaries ([3e664a7](https://github.com/dnaka91/wazzup/commit/3e664a7fbde576eefb9b127abb602544e04a4b03))

## [0.3.3](https://github.com/dnaka91/wazzup/compare/v0.3.2...v0.3.3) - 2023-07-08

### <!-- 1 -->🐛 Bug Fixes

- Shutdown WebSocket connection on shutdown to prevent hanging ([fcb003c](https://github.com/dnaka91/wazzup/commit/fcb003ce7c21c9db03963c8a044ba54ae351d002))
- Don't wait forever for the client to receive the reload signal ([e7dbc36](https://github.com/dnaka91/wazzup/commit/e7dbc36852cd6341e7665627f89997e3bd20cc4d))

### <!-- 7 -->⚙️ Miscellaneous Tasks

- Update dependencies ([3295f4c](https://github.com/dnaka91/wazzup/commit/3295f4ce4c27f3df6d52c941403432af886dc98b))

## [0.3.2](https://github.com/dnaka91/wazzup/compare/v0.3.1...v0.3.2) - 2023-06-07

### <!-- 1 -->🐛 Bug Fixes

- Search the right binary ([b3fb391](https://github.com/dnaka91/wazzup/commit/b3fb3919c771417c67053316809b4d8f0368ad18))
  > After the previous change of improving error messages for missing
  > binaries, the lookup function always search for tailwindcss instead
  > of the passed binary name.

## [0.3.1](https://github.com/dnaka91/wazzup/compare/v0.3.0...v0.3.1) - 2023-06-07

### <!-- 0 -->⛰️ Features

- Give better error messages for missing binaries ([117adf2](https://github.com/dnaka91/wazzup/commit/117adf264515050c8b76a98e581389797c004609))
- Log the watched file paths in verbose mode ([49a4fe1](https://github.com/dnaka91/wazzup/commit/49a4fe146cfc4ff3b129c103d70d03cb5017a619))

### <!-- 7 -->⚙️ Miscellaneous Tasks

- Update dependencies ([95f9432](https://github.com/dnaka91/wazzup/commit/95f9432f08317c220703ceb1138c9f8cfe2fbee6))
- Adjust rustfmt code formatting settings ([c044c61](https://github.com/dnaka91/wazzup/commit/c044c616733a950093c34abb461fc58d9a1617b9))
- Update dependencies ([a3245d4](https://github.com/dnaka91/wazzup/commit/a3245d48d46ca251372a21f58c8fefe17f1ce50d))

## [0.3.0](https://github.com/dnaka91/wazzup/compare/v0.2.0...v0.3.0) - 2023-04-21

### <!-- 0 -->⛰️ Features

- Support projects inside a workspace ([1911a84](https://github.com/dnaka91/wazzup/commit/1911a84025d070a2e5a0120c18f64321a39834fb))
- Add support for TailwindCSS ([0782bae](https://github.com/dnaka91/wazzup/commit/0782bae33f552fea89d417fd39f30924ab88fbaf))

### <!-- 7 -->⚙️ Miscellaneous Tasks

- Update `minify-js` to v0.5 ([5072ddc](https://github.com/dnaka91/wazzup/commit/5072ddc864c4bbd58f5d679e3d2cb2ea59711014))

## [0.2.0](https://github.com/dnaka91/wazzup/compare/v0.1.0...v0.2.0) - 2023-04-18

### <!-- 0 -->⛰️ Features

- Build WASM binary as last step ([c9e1193](https://github.com/dnaka91/wazzup/commit/c9e119357f4bb758880d3150ed460a1ae748b398))
- Minify HTML and JS files ([368262f](https://github.com/dnaka91/wazzup/commit/368262f8601a959b821f7cc0c633d40ea9077413))

### <!-- 1 -->🐛 Bug Fixes

- Use same file walk settings for minify step ([fa61ec9](https://github.com/dnaka91/wazzup/commit/fa61ec9e8d276143819607eee6f5b1e04d04a88c))
- Ignore .git folders in the file watcher ([726af9d](https://github.com/dnaka91/wazzup/commit/726af9d2f8bc24964ce1945ed3a95666b82ef27e))

### <!-- 4 -->🚜 Refactor

- Run wasm-opt as part of minify ([591f341](https://github.com/dnaka91/wazzup/commit/591f34125bc1a518535b4b6eab2de6aa51fefc8c))

### <!-- 7 -->⚙️ Miscellaneous Tasks

- Update dependencies ([3af34d1](https://github.com/dnaka91/wazzup/commit/3af34d11f2c47517347002d49bf145f9208038e3))

## [0.1.0](https://github.com/dnaka91/wazzup/releases/tag/v0.1.0) - 2023-02-11

### <!-- 7 -->⚙️ Miscellaneous Tasks

- Initial commit ([206e4af](https://github.com/dnaka91/wazzup/commit/206e4afefdbde6cdce429b82304005500c443455))
<!-- generated by git-cliff -->

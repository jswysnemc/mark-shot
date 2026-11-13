# Development

**English** | [简体中文](zh-CN/development.md)

## Repository layout

```text
src/
  main.rs, cli/        Entry point, argument parsing, command dispatch
  app/                 Capture session: input dispatch, frame pacing, export flow
  backend/             Platform backends
    wayland/           Capture protocols, overlay, windows, pins, clipboard, text input
    x11/               X11 capture, windows, clipboard, XIM
    windows/           Win32 capture, windows, clipboard
    window_detection/  Window geometry sources: niri, Sway, X11, Windows
  editor/              Editing state, undo history, damage tracking, caches
  renderer/            Annotation, text and pixel compositing; output images
  tools/               Input behaviour of each annotation tool
  features/            Feature modules: recording, OCR result window, history, pins,
                       preferences, code scanning, desktop integration, and each annotation type
  ui/                  Toolbar, property panels, popovers, text editor, file picker
  ocr/                 Models, downloads, inference, daemon
  translation/         Translation providers and tasks
  config/              Configuration types, settings registry, persistence, legacy formats
  upload/              HTTP and command upload
crates/
  shot-image/          Image buffers, blur, pixelation, colour filters, export frame
  shot-stitch/         Scrolling screenshot stitching
  shot-media/          Recording and export through a private C library (native/*.c)
assets/                Fonts, icons, application icon
packaging/, debian/    Package definitions
scripts/               Packaging, media library build, verification tools
tests/                 Integration tests and fixtures
docs/                  Published documentation
```

`.docs/` holds working notes produced during development. It is ignored by git and is not part of
the published documentation.

## Architecture in brief

- **Software rendering.** The interface is drawn with tiny-skia and cosmic-text into shared
  memory buffers. There is no GPU interface runtime and no widget toolkit. The frozen screenshot
  and the dynamic annotations are separate layers, and only damaged regions are redrawn.
- **Backends behind traits.** `src/backend.rs` defines capture, overlay, window and clipboard
  contracts. Feature code does not talk to a Wayland or X11 connection directly, and a backend
  reports a missing capability as an error instead of emulating it.
- **One executable, several processes.** Pinned images, the clipboard provider, the OCR daemon,
  the OCR result window, the history window, the settings window and the recording controller are
  separate processes of the same binary, started with internal arguments. Closing the editor
  therefore releases its buffers and any OCR model.
- **Idle means blocked.** Event loops block when there is no input or pending work. Timers exist
  only while something is animating or counting down.
- **Resources on demand.** A plain screenshot initializes the backend, the image buffers and the
  interface font. OCR, upload and translation are created the first time they are used.
- **Bounded memory.** Caches have explicit limits, for example 4 MiB per redaction region and
  16 MiB in total. Long screenshots are grown in fixed row blocks and encoded line by line.

## Building

See [Installation](installation.md) for dependencies and build commands. A debug build with
`cargo build` is enough for most work; dependencies are optimized in the `dev` profile.

## Checks

A change is expected to pass the same checks as the `Native Rust Build` workflow:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Additional checks, depending on what was touched:

| Area | Command |
| --- | --- |
| OCR with WebGPU (x86_64) | `cargo clippy --workspace --all-targets --no-default-features --features ocr-gpu --locked -- -D warnings` |
| Windows | `cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc --locked -- -D warnings` |
| Python verification tools | `uv run --with pillow --with numpy python -m unittest discover -s scripts -p 'test_*.py'` |
| Packaging scripts | `uv run --with pyyaml python -m unittest discover -s tests/packaging -v` |
| Workflows | `bash scripts/packaging/check_workflows.sh` |
| Media library | `MARK_SHOT_MEDIA_LIBRARY="$PWD/target/media-linux/libmarkshot-media.so" cargo test -p shot-media --locked -- --ignored` |

Some tests are marked `#[ignore]` because they need external resources such as OCR models, a
network service or the media library.

### Interface tests

The `scripts/wayland-*.py` and `scripts/x11-*.py` tools drive the real application inside a
private, invisible session, so they do not touch your desktop. `scripts/wayland_test_runtime.py`
starts a nested niri inside Xvfb and runs one tool in it:

```sh
cargo build --release --locked
uv run --no-project --python /usr/bin/python3 python scripts/wayland_test_runtime.py -- \
  uv run --no-project --python /usr/bin/python3 python scripts/wayland-arrow.py
```

They need niri, Xvfb, niri-use, xdotool, wtype, grim and Pillow. These are test dependencies only.

## Conventions

- Keep a source file under 750 lines and give it one responsibility. Split by responsibility
  before a file grows past that.
- Doc comments on functions state the purpose, the parameters and the return value. Comments and
  log messages of a larger flow share a `【module】【action】` prefix so they can be searched.
- Log with `info` or `error` only.
- A capability that a platform does not offer returns an error that names the reason. Do not add
  an empty implementation that pretends to work.
- Commit messages follow Conventional Commits: `feat:`, `fix:`, `docs:`, `test:`, `ci:`.

## Packaging

`scripts/packaging/build_linux.sh` builds the executable and the media library, checks the dynamic
linkage and collects the dependency licenses. `scripts/packaging/install_linux.py` then creates the
install tree:

```sh
bash scripts/packaging/build_linux.sh
python3 scripts/packaging/install_linux.py \
  --destination "$PWD/target/stage" --source-sha "$(git rev-parse HEAD)"
```

The per-format scripts `package_deb.py`, `package_rpm.py`, `package_pacman.py`,
`package_appimage.py`, `package_flatpak.py` and `package_portable.py` turn that tree into a
package. `native-release.yml` handles `native-v*` tags, calls the existing `release-*.yml`
workflows and waits for all verification and package jobs before publishing. `windows-build.yml`
produces the Windows archive. Per-format manual runs produce Actions artifacts rather than a
partial GitHub Release. Stable AUR metadata publication requires an explicit manual request.

Add the update description at `release/notes/<version>.md` before creating a tag. The publisher
creates a draft, verifies every uploaded package, and then exposes the release with a collapsed
download list and SHA-256 values. Checksum sidecars and JSON manifests stay in Actions artifacts.
Published filenames normalize `~` and `+` to `.`, without changing the package contents.

If only publication fails, reuse the verified build without rebuilding or moving its tag:

```sh
gh workflow run native-release-publish.yml --ref main \
  -f tag=native-v0.2.0-preview.7 -f run_id=43512259674
```

The retry workflow checks the source repository, exact tag commit, original release workflow
and every required build group before downloading its artifacts. It resumes the existing draft
and rejects assets with different contents. Prereleases do not update stable AUR or PPA channels.

Distribution builds run offline: `prepare_source.py` vendors the Rust crates and the media library
sources into the source archive, and `build_linux.sh --offline` uses only those.

## Documentation

- Published documentation lives in `docs/`, with the Chinese version of every page under
  `docs/zh-CN/`. Update both when behaviour changes.
- `README.md` is English and links to `README.zh-CN.md`.
- State only what the code does. If something is unverified on a platform, say so.

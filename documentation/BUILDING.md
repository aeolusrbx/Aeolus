# Building Aeolus

## On Windows (recommended)

Requirements:

- Node.js 20 or newer
- Rust (stable) through [rustup](https://rustup.rs) with the MSVC toolchain
- Microsoft C++ Build Tools ("Desktop development with C++")
- WebView2 Runtime (preinstalled on Windows 10 21H2+ and Windows 11)

```powershell
npm install
npm run tauri dev        # development build with hot reload
npm run tauri build      # release: src-tauri\target\release\bundle\{nsis,msi}\
```

`tauri.conf.json` builds both an **NSIS** setup (`*-setup.exe`) and an **MSI**. Building the MSI needs WiX, which the Tauri CLI downloads automatically on Windows.

## Cross-compiling from Linux

The 0.1.0 release was produced this way. Install `mingw-w64` and `nsis`, then:

```bash
rustup target add x86_64-pc-windows-gnu
npx tauri build --target x86_64-pc-windows-gnu --bundles nsis
```

`src-tauri/.cargo/config.toml` points that target at the MinGW linker. MSI cannot be produced on Linux because WiX is Windows-only.

If `rustup target add` is unavailable (for example on an offline mirror), you can build the standard library from source instead: install the matching `rust-src`, then run `RUSTC_BOOTSTRAP=1 npx tauri build --target x86_64-pc-windows-gnu --bundles nsis -- -Zbuild-std=std,panic_abort`.

## Tests

```bash
npm test                      # Vitest: UI flows against an in-memory backend (tests/mockBackend.ts)
npm run typecheck             # TypeScript, strict
cd src-tauri && cargo test    # Rust unit tests + tests/install_pipeline.rs
```

`tests/install_pipeline.rs` runs the real installer pipeline against a local HTTP server that imitates Roblox's CDN. It covers interrupted downloads with Range resume, checksum mismatch, cancellation, unknown packages, verify and tamper detection, repair, and the refusal to delete folders Aeolus didn't create.

## Previewing the UI in a browser

```bash
VITE_MOCK=1 npx vite
```

This runs the interface against the same in-memory mock backend the tests use. The mock is excluded from release builds.

# Wardwatch

Wardwatch watches for critical vulnerabilities on your devices and helps you update promptly.

## Layout

| Path                     | What it is                                                                                               |
| ------------------------ | -------------------------------------------------------------------------------------------------------- |
| `apps/desktop`           | The desktop app: React + TypeScript UI (Vite, Tailwind, React Router, TanStack Query)                    |
| `apps/desktop/src-tauri` | The Tauri shell in Rust: main window, menu bar / tray popover, IPC commands                              |
| `packages/*`             | Shared TypeScript packages, such as the design system (`@wardwatch/ui`)                                  |
| `crates/*`               | Rust libraries with no Tauri dependency, such as the software catalog. Add each to the root `Cargo.toml` |
| `packaging/linux`        | Build, assemble and test the `.deb`/`.rpm` and the apt/dnf repo trees, locally and unsigned              |

The UI talks to Rust only through commands in `apps/desktop/src-tauri/src/commands.rs`, wrapped in `apps/desktop/src/lib/ipc.ts`. Each command is listed in `build.rs` and granted per window in `src-tauri/capabilities/`, so a window can call only what it needs.

## Develop

You need Node 22, pnpm 10 and stable Rust (1.90 or newer). On Linux, also install the [Tauri system libraries](https://v2.tauri.app/start/prerequisites/#linux).

```sh
pnpm install
pnpm --filter @wardwatch/desktop tauri dev   # run the app
pnpm --filter @wardwatch/desktop dev         # UI only, in a browser
```

Findings and the fix screen need a vulnerability catalog. Catalogs are unsigned until Wardwatch's signing keys exist, so only development builds load one: put the backend's `catalog.json` in the app's data folder, or point `WARDWATCH_CATALOG` at it. Release builds refuse every catalog for now. In a plain browser the UI uses the design spec's sample scenario instead.

## Check

```sh
pnpm format:check && pnpm lint && pnpm typecheck && pnpm test
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
pnpm --filter @wardwatch/desktop tauri build  # unsigned local bundles in target/release/bundle
```

CI runs the same checks and builds unsigned bundles on macOS, Windows and Linux. It never signs, uploads or publishes builds.

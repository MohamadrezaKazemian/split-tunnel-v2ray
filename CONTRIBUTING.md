# Contributing

Install Node.js 22, pnpm 12, stable Rust, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). Linux also needs `pkexec` and a running polkit agent. Then run:

```sh
pnpm install --frozen-lockfile
pnpm setup:engine
pnpm tauri dev
```

Run `pnpm lint`, `pnpm typecheck`, `pnpm test`, `cargo fmt --manifest-path src-tauri/Cargo.toml --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, and `cargo test --manifest-path src-tauri/Cargo.toml` before submitting. Use `pnpm tauri build` for a native build and follow the [Windows test plan](docs/WINDOWS_TEST_PLAN.md) for Windows changes.

Follow existing TypeScript and Rust style; use `cargo fmt` for Rust. Keep credentials and generated binaries out of commits. Use [Conventional Commits](https://www.conventionalcommits.org/), such as `fix: handle missing engine`. Open an issue to discuss substantial changes, create a branch and focused pull request, describe the change and tests, and address review feedback. By contributing, you agree that your contribution may be distributed under the project's MIT license. Follow the code of conduct.

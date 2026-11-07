# Third-party notices

This project is MIT licensed; third-party components retain their own licenses.
See the lockfiles and the upstream projects for the complete dependency graph,
license texts, notices, and the exact versions used by a build.

## sing-box

The app downloads and bundles the official **unmodified sing-box 1.14.4** executable.
Its [source for v1.14.4](https://github.com/SagerNet/sing-box/tree/v1.14.4),
[source archive](https://github.com/SagerNet/sing-box/archive/refs/tags/v1.14.4.tar.gz),
[release](https://github.com/SagerNet/sing-box/releases/tag/v1.14.4), and
[license notice](https://github.com/SagerNet/sing-box/blob/v1.14.4/LICENSE)
are available upstream. Its notice states GPL-3.0-or-later and includes an
additional restriction on use of its name or implying association in derivative
works. The GPL applies to sing-box and its covered components; the MIT license
for this project's own code does not replace their terms. Distributors must
satisfy applicable source, license, and notice obligations, including those of
sing-box's dependencies. Review the actual distribution and corresponding source
before publishing binaries.

## Wintun

The upstream Windows sing-box binary incorporates Wintun. [Wintun source and
license information](https://git.zx2c4.com/wintun/about/) and the
[official binary distribution](https://www.wintun.net/) explain that prebuilt
signed DLLs have separate terms included in their archive. Check the precise
upstream artifact and its included notices before redistributing it; no standalone
Wintun binary is downloaded by this project.

## Main dependencies

- [Tauri and its CLI/API/dialog plugin](https://github.com/tauri-apps/tauri): MIT OR Apache-2.0.
- [React and React DOM](https://github.com/facebook/react): MIT.
- [TypeScript](https://github.com/microsoft/TypeScript): Apache-2.0.
- [Vite](https://github.com/vitejs/vite), [Vitest](https://github.com/vitest-dev/vitest), and [jsdom](https://github.com/jsdom/jsdom): MIT.
- [Serde and serde_json](https://github.com/serde-rs/serde), [reqwest](https://github.com/seanmonstar/reqwest), [libc](https://github.com/rust-lang/libc), and [windows-sys](https://github.com/microsoft/windows-rs): MIT OR Apache-2.0.
- [sysinfo](https://github.com/GuillaumeGomez/sysinfo): MIT; [shell-words](https://github.com/tmiasko/shell-words): MIT OR Apache-2.0.

Transitive dependencies can have additional terms. Consult `pnpm-lock.yaml`,
`src-tauri/Cargo.lock`, and the upstream sing-box distribution when assembling releases.

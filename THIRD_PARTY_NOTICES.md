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

## DejaVu Sans font

The Classic theme bundles the unmodified [DejaVu Sans](https://dejavu-fonts.github.io/) font. DejaVu changes are in the public domain; the original Bitstream Vera font is distributed under the following license:

Copyright (c) 2003 by Bitstream, Inc. All Rights Reserved. Bitstream Vera is a trademark of Bitstream, Inc.

Permission is hereby granted, free of charge, to any person obtaining a copy of the fonts accompanying this license ("Fonts") and associated documentation files (the "Font Software"), to reproduce and distribute the Font Software, including without limitation the rights to use, copy, merge, publish, distribute, and/or sell copies of the Font Software, and to permit persons to whom the Font Software is furnished to do so, subject to the following conditions:

The above copyright and trademark notices and this permission notice shall be included in all copies of one or more of the Font Software typefaces.

The Font Software may be modified, altered, or added to, and in particular the designs of glyphs or characters in the Fonts may be modified and additional glyphs or characters may be added to the Fonts, only if the fonts are renamed to names not containing either the words "Bitstream" or the word "Vera".

This License becomes null and void to the extent applicable to Fonts or Font Software that has been modified and is distributed under the "Bitstream Vera" names.

The Font Software may be sold as part of a larger software package but no copy of one or more of the Font Software typefaces may be sold by itself.

THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF COPYRIGHT, PATENT, TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL BITSTREAM OR THE GNOME FOUNDATION BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, INCLUDING ANY GENERAL, SPECIAL, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL DAMAGES, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF THE USE OR INABILITY TO USE THE FONT SOFTWARE OR FROM OTHER DEALINGS IN THE FONT SOFTWARE.

Except as contained in this notice, the names of Gnome, the Gnome Foundation, and Bitstream Inc., shall not be used in advertising or otherwise to promote the sale, use or other dealings in this Font Software without prior written authorization from the Gnome Foundation or Bitstream Inc., respectively. For further information, contact: fonts at gnome dot org.

Transitive dependencies can have additional terms. Consult `pnpm-lock.yaml`,
`src-tauri/Cargo.lock`, and the upstream sing-box distribution when assembling releases.

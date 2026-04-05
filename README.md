# vscode-mcshader

vscode-mcshader is a new [Language Server](https://microsoft.github.io/language-server-protocol/) borns from [Strum355/mcshader-lsp](https://github.com/Strum355/mcshader-lsp/) with rewrited server side part, introducing lots of new LSP features that make your Minecraft shader developing experience better.

This extension can load a platform-specific language server binary from `server/bin/<platform>-<arch>/`.

## License

Part of code is released under the [MIT License]. Copyright (c) 2021 Noah Santschi-Cooney

Most code is released under the [MIT License]. Copyright (c) 2023 GeForceLegend

Work spaces support idea from Fayer3

## Features

 - Real-time linting with optifine builtin macro support;
 - Include document links;
 - Multiple work space or multiple shader folders in one work space;
 - Temporary linting and document link for files outside work space (temporary linting only supports base shader file);
 - Virtual merge for base shader file;
 - File watcher for file changes (creating, deleting, etc). By default it supports files with `[csh, vsh, gsh, fsh, tcs, tes, glsl]` and `inc` (extra) extensions, you can add more by extension configuration;
 - Single-file goto-definitions and references;
 - Document symbols provider;
 - Workspace edits for include macro when renaming files.

This extension does not provide syntax highlight for GLSL yet. If you want GLSL syntax highlight, you can install this extension with [vscode-glsl](https://github.com/GeForceLegend/vscode-glsl) or [vscode-shader](https://github.com/stef-levesque/vscode-shader).

## Known issue

 - Code like this will disable inserted `#line` macro, and let the rest of this file reporting wrong line if error occured, unless found another active `#include`. To avoid this issue, please place an include that is always active behind it.
```glsl
#ifdef A // A is not defined defaultly
#include "B"
#endif

// To avoid this issue, please add an active include here before writing other code.
```

## Build and use guide

 - Run `npm install` in the repo root
 - Run `npm run build:extension` in the repo root if you want to build without packaging
 - This builds the Rust server in release mode, builds the extension bundle, and copies the server output to `server/bin/<platform>-<arch>/vscode-mcshader(.exe)`
 - Run `npm run package` in the repo root to build and create the `.vsix`
 - If you already built everything and only want to package existing artifacts, run `vsce package` in the repo root
 - Open VS Code and install the generated `.vsix`

## Multi-platform packaging

 - The extension now resolves the server binary by `process.platform` and `process.arch`
 - Example folders: `server/bin/win32-x64/`, `server/bin/linux-x64/`, `server/bin/darwin-arm64/`
 - A single VSIX can support multiple platforms if you place every matching binary in those folders before packaging
 - If you only package one platform binary, the VSIX will still only work on that platform

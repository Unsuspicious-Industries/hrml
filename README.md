# XRML

`xrml` is the Rust CLI for HRML sites: HTML-first templates, server-side
rendering and Python endpoints. HRML names the templating language; XRML names
the executable. The CLI reads `xrml.toml`.

## Run from Nix

```sh
nix run github:Unsuspicious-Industries/hrml -- --help
nix run github:Unsuspicious-Industries/hrml -- new myapp
cd myapp
nix run github:Unsuspicious-Industries/hrml -- dev
```

The flake's default app and `packages.x86_64-linux.xrml` select the same
`bin/xrml`. The current flake supports `x86_64-linux` only. Pin a Git revision
when selecting the source for an experiment or deployment.

From a source checkout, `cargo build --release` produces
`target/release/xrml`, not `target/release/hrml`.

## Commands

| Command | Purpose |
|---|---|
| `xrml new NAME` | Create a project |
| `xrml dev [PATH]` | Develop with source reloads |
| `xrml serve [PATH]` | Serve the project's source |
| `xrml build [PATH]` | Export a static site into `dist` |
| `xrml check [PATH]` | Diagnose template and configuration errors |
| `xrml convert [FILE]` | Convert HRML syntax to TRML syntax |
| `xrml version` | Report the executable's version |
| `xrml help` | Show options and usage |

Path commands use the current directory when PATH is omitted. `--palette FILE`
resolves palette tokens; `serve` also accepts `--host ADDRESS` and `--port PORT`.
Use `--debug` for render diagnostics or `--log-ast` for an AST log.

## Project contract

```text
myapp/
├── xrml.toml
├── templates/
│   ├── layouts/base.hrml
│   ├── components/
│   └── pages/index.hrml
├── endpoints/api/
└── static/
```

[spec.md](spec.md) is the authoritative description of configuration,
templates, routing, Python endpoints and rendering behavior. Keep those
contracts there rather than repeating them in installation instructions.

For development, run `xrml dev` from the project directory. For source serving,
run `xrml serve`; for static hosting, publish the output of `xrml build`.
Source serving reloads after 500 ms without a relevant change. This trailing
edge prevents a reset burst's first event from being the only reload. Reload
failures retain the current project; index-render validation does not establish
validity of every route or an atomic Git snapshot.
On a NixOS host, declare the package and service rather than copying a binary
into `/usr/local/bin`.

## License

MIT; see [LICENSE](LICENSE).

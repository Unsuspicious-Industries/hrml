# Unsuspicious Industries — Website

The [unsuspicious.org](https://unsuspicious.org) website, built with [HRML](https://github.com/Unsuspicious-Industries/hrml).

## Prerequisites

Install the `xrml` build tool:

```bash
cargo install xrml
```

Or from source:

```bash
git clone https://github.com/Unsuspicious-Industries/hrml
cd hrml
cargo install --path .
```

## Development

```bash
xrml dev
```

Opens a dev server at `http://localhost:8080` with auto-reload.

## Build

```bash
xrml build
```

Static output goes to `dist/`.

## Project Structure

```
usi/
├── xrml.toml                 # Project config
├── templates/
│   ├── layouts/base.hrml     # Master layout
│   ├── components/            # Reusable components (auto-registered)
│   └── pages/                 # Page templates → routes
├── data/
│   ├── posts/                 # Blog MDX posts
│   ├── jobs/                  # Job listing JSON
│   ├── notes/                 # Research notes MDX
│   └── resources/             # Reference resources TOML
└── static/
    ├── css/                   # Global styles
    ├── images/                # Images
    ├── fonts/                 # Web fonts
    └── icons/                 # SVG icons
```

## Adding Pages

1. Create a `.hrml` file in `templates/pages/`
2. Add a nav link in `templates/components/nav.hrml`
3. Rebuild

## License

MIT

# usi

A web application built with HRML - Minimal Web Framework

## Project Structure

```
usi/
├── hrml.toml              # Configuration
├── templates/             # HTML templates
│   ├── layouts/          # Layout templates
│   ├── components/       # Reusable components
│   └── pages/            # Page templates
└── static/               # Static assets
    ├── css/              # Stylesheets
    ├── js/               # JavaScript files
    └── images/           # Images
```

## Development

```bash
# Run development server with auto-reload
hrml dev

# Or serve from this directory
hrml serve
```

## Building for Production

```bash
# Build static site
hrml build

# Output will be in the `dist/` directory
```

## Adding Pages

1. Create a new template in `templates/pages/`
2. Link to it from navigation in `templates/components/nav.hrml`

See the HRML documentation for more details.

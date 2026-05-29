# HRML Syntax Highlighting for VS Code

Provides syntax highlighting for HRML template files (.hrml).

## Features

- Syntax highlighting for HRML processing instructions
- Support for embedded CSS and JavaScript
- Auto-closing pairs for HRML tags
- Bracket matching and folding

## Installation

Copy this directory to your VS Code extensions folder:
- Windows: `%USERPROFILE%\.vscode\extensions`
- macOS/Linux: `~/.vscode/extensions`

Or package and install:
```bash
npm install -g vsce
vsce package
code --install-extension hrml-syntax-0.1.0.vsix
```

## Supported Tags

- `<?load?>` - Template inclusion
- `<?set?>` / `<?get?>` - Data binding
- `<?if?>` / `<?else?>` - Conditionals
- `<?for?>` - Loops
- `<?slot?>` / `<?block?>` - Layouts
- `<?btn?>` / `<?link?>` / `<?form?>` - HTMX wrappers
- `<?call?>` - Endpoint calls
- `<?style?>` / `<?script?>` - Embedded CSS/JS

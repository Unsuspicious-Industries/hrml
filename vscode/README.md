# HRML Syntax Highlighting for VS Code

Provides syntax highlighting for HRML template files (.hrml).

## Features

- Syntax highlighting for every HRML processing instruction - built-in
  directives are coloured as keywords; any other `<?name?>` (a named-tag
  component prop, e.g. `<?title?>…<?/title?>`) is coloured as a tag
- Both closing spellings: `</?name?>` and `<?/name?>`
- `$variable` references (including dotted paths like `$post.title`),
  highlighted in text content and inside directive attribute values
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

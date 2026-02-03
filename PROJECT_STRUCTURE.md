# HRML Project Structure Documentation

## Overview

HRML is a minimal web framework combining Rust, Python, and custom JavaScript. Projects follow a strict structure that the `hrml init` command enforces.

## Project Structure

When you run `hrml new <project-name>`, the following structure is created:

```
project-name/
├── hrml.toml              # Configuration file
├── README.md              # Project documentation
├── .gitignore            # Git ignore patterns
├── templates/            # HTML templates
│   ├── layouts/          # Layout templates (required)
│   │   └── base.hrml     # Base layout (required)
│   ├── components/       # Reusable components
│   │   └── nav.hrml      # Navigation component
│   └── pages/            # Page templates
│       ├── index.hrml    # Home page (required)
│       └── about.hrml    # About page
├── endpoints/            # Python backend endpoints
│   ├── __init__.py       # Makes it a Python package
│   └── api/              # API endpoints
│       ├── __init__.py   # Makes it a Python package
│       └── hello.py      # Sample endpoint
└── static/               # Static assets
    ├── css/              # Stylesheets
    │   └── style.css     # Default styles
    ├── js/               # JavaScript files
    └── images/           # Images
```

## Required Files

### hrml.toml (Configuration)

The configuration file that HRML reads on startup:

```toml
[project]
name = "my-project"
version = "0.1.0"

[server]
host = "127.0.0.1"
port = 8080

[paths]
templates = "templates"
endpoints = "endpoints"
static = "static"

[site]
name = "My Project"
description = "A web application"
favicon = "/static/favicon.ico"
```

**Behaviors:**
- All paths are relative to the project root
- Server host defaults to "127.0.0.1" if not specified
- Server port defaults to 8080 if not specified
- If hrml.toml is missing, HRML uses defaults

### templates/layouts/base.hrml (Required)

The base layout template that all pages extend:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title><?get id="site_name"?></title>
    <link rel="stylesheet" href="/static/css/style.css">
    <script src="/hrml.js"></script>
</head>
<body>
    <?load file="components/nav.hrml"?>
    
    <main class="container">
        <?slot id="content"?>
    </main>
    
    <footer>
        <p>&copy; 2024 <?get id="site_name"?></p>
    </footer>
</body>
</html>
```

**Template Directives:**
- `<?load file="path/to/file.hrml"?>`: Include another template
- `<?slot id="name"?>`: Define a content slot that pages can fill
- `<?block slot="name"?>...<?/block?>`: Fill a slot from a parent layout
- `<?set id="name"?>value<?/set?>`: Set a variable
- `<?get id="name"?>`: Get a variable value
- `<?if cond="condition"?>...<?/if?>`: Conditional rendering
- `<?for item in="items"?>...<?/for?>`: Loop over items

### templates/pages/index.hrml (Required)

The home page template:

```html
<?load file="layouts/base.hrml"?>
<?block slot="content"?>
    <h1>Welcome</h1>
    <p>This is the home page.</p>
<?/block?>
```

**Page Structure:**
1. Must load a layout using `<?load file="layouts/base.hrml"?>`
2. Must define content blocks using `<?block slot="content"?>`
3. All HTML content goes inside blocks

## Python Endpoints

### Structure

Endpoints are Python files in the `endpoints/api/` directory:

```python
# endpoints/api/hello.py
def handler(req):
    """
    Handle API requests.
    
    Args:
        req: Dictionary with keys:
            - 'id': Resource ID (for /api/resource/123)
            - 'action': Action name (for /api/resource/action)
            - 'data': Form data as dictionary
    
    Returns:
        HTML string to be inserted into the page
    """
    action = req.get('action', '')
    data = req.get('data', {})
    
    if action == 'create':
        # Handle create
        return "<div>New item created</div>"
    
    # Default: return list or single item
    return "<div>Hello from Python</div>"
```

### URL Routing

Endpoints are automatically routed based on file structure:

- `/api/hello` → calls `endpoints/api/hello.py::handler()`
- `/api/todos/123` → calls `endpoints/api/todos.py::handler()` with id="123"
- `/api/todos/123/delete` → calls handler with id="123", action="delete"
- `/api/todos/create` → calls handler with action="create"

### Database Access

HRML provides a `db` module with table operations:

```python
import db
import json

# Create a table (if not exists)
db.table_create("todos", "id INTEGER PRIMARY KEY, title TEXT, done INTEGER")

# Insert data (returns ID)
todo_id = db.table_insert("todos", json.dumps({"title": "Buy milk", "done": 0}))

# Find by ID
result = db.table_find("todos", todo_id)  # Returns JSON string
item = json.loads(result)

# Get all records
results = db.table_find_all("todos")  # Returns JSON array string
items = json.loads(results)

# Update by ID
db.table_update("todos", todo_id, json.dumps({"done": 1}))

# Delete by ID
db.table_delete("todos", todo_id)
```

## Static Assets

### CSS

Place CSS files in `static/css/`:
- `static/css/style.css` - Default stylesheet (loaded by base layout)
- Additional CSS files can be created and linked

### JavaScript

Place JS files in `static/js/`:
- HRML provides `/hrml.js` automatically (no need to create)
- Add custom scripts and link them in templates

### Images

Place images in `static/images/`:
- Reference as `/static/images/logo.png`

## Template Directives Reference

### `<?load file="path"?>`

Includes another template file.

**Path resolution:**
- Paths are relative to `templates/` directory
- No leading slash needed
- Example: `<?load file="components/nav.hrml"?>`

### `<?slot id="name"?>`

Defines a content placeholder in a layout.

**Usage in layout:**
```html
<main>
    <?slot id="content"?>
</main>
```

**Behavior:**
- Slots must be filled by child templates using `<?block?>`
- Slots can have default content: `<?slot id="sidebar"?>Default<?/slot?>`

### `<?block slot="name"?>...<?/block?>`

Fills a slot defined in a parent layout.

**Usage:**
```html
<?load file="layouts/base.hrml"?>
<?block slot="content"?>
    <h1>Page Title</h1>
    <p>Page content here</p>
<?/block?>
```

### `<?set id="name"?>value<?/set?>`

Sets a variable value.

**Usage:**
```html
<?set id="page_title"?>About Us<?/set?>
<h1><?get id="page_title"?></h1>
```

### `<?get id="name"?>`

Retrieves a variable value.

**Behavior:**
- Looks for variable in current scope
- If not found, returns empty string
- Special variables: `site_name`, `site_description`

### `<?if cond="condition"?>...<?/if?>`

Conditional rendering (future feature).

### `<?for item in="items"?>...<?/for?>`

Loop over collection (future feature).

## Client-Side JavaScript (HRML Runtime)

HRML automatically serves `/hrml.js` which provides HTMX-like functionality:

### data-post

POST to an endpoint and update the DOM:

```html
<button data-post="/api/counter/increment" 
        data-target="#counter-display"
        data-swap="innerHTML">
    Click Me
</button>
```

**Attributes:**
- `data-post`: URL to POST to
- `data-target`: CSS selector for element to update
- `data-swap`: How to insert content (innerHTML, outerHTML, beforeend)

### data-get

GET from an endpoint and update the DOM:

```html
<a href="/about" 
   data-get="/api/content/about"
   data-target="#content"
   data-swap="innerHTML">
   About
</a>
```

### data-trigger="load"

Auto-load content when page loads:

```html
<div data-get="/api/todos" 
     data-trigger="load"
     data-swap="innerHTML">
    Loading...
</div>
```

### Form submissions

Forms with `data-post` submit via AJAX:

```html
<form data-post="/api/todos/create"
      data-target="#todo-list"
      data-swap="beforeend">
    <input name="title" required>
    <button type="submit">Add</button>
</form>
```

**Behavior:**
- Form data is serialized and sent as POST body
- Response HTML is inserted at target
- Form is reset after successful submission

## Development Workflow

### 1. Create New Project

```bash
hrml new myapp
cd myapp
```

### 2. Start Development Server

```bash
hrml dev
```

**Behaviors:**
- Watches for file changes (templates, endpoints, static)
- Auto-reloads on changes
- Shows detailed error messages in browser
- Logs all requests

### 3. Add Pages

1. Create template in `templates/pages/`
2. Link in navigation: `templates/components/nav.hrml`
3. Access at `http://localhost:8080/page-name`

### 4. Add Endpoints

1. Create Python file in `endpoints/api/`
2. Define `handler(req)` function
3. Access at `http://localhost:8080/api/filename`

### 5. Validate Project

```bash
hrml check
```

**Checks:**
- hrml.toml exists and is valid
- Required templates exist
- Template engine can be initialized
- Static directory exists

## Production Deployment

### Option 1: Binary Deployment

Build the HRML binary and deploy with your project:

```bash
# Build release binary
cargo build --release

# Deploy binary + project files
./hrml serve /path/to/project
```

### Option 2: Static Site Generation (Future)

```bash
hrml build
# Generates static files in dist/
```

## Error Handling

### Template Errors

If a template fails to render:
- Development: Detailed error message shown
- Production: 500 error with generic message

### Endpoint Errors

If a Python endpoint fails:
- Error logged to stderr
- 500 response with error message
- Request data preserved for debugging

### Missing Files

If required files are missing:
- Server logs warning on startup
- Request returns 404
- Validation tool reports missing files

## Configuration Options

### hrml.toml

```toml
[project]
name = "my-app"                    # Project name
version = "1.0.0"                  # Project version

[server]
host = "127.0.0.1"                 # Bind address
port = 8080                        # Bind port
workers = 4                        # Future: Number of workers

[paths]
templates = "templates"            # Templates directory
endpoints = "endpoints"            # Endpoints directory
static = "static"                  # Static files directory

[site]
name = "My Application"            # Site title
description = "Description"        # Meta description
favicon = "/static/favicon.ico"    # Favicon path

[database]
path = "app.db"                    # Future: SQLite database path

[features]
hot_reload = true                  # Future: Enable hot reload
compression = true                 # Future: Enable gzip
```

## Best Practices

### Template Organization

- Use layouts for consistent structure
- Create reusable components
- Keep pages focused on specific content
- Use descriptive names for slots

### Endpoint Design

- One endpoint file per resource type
- Use actions for different operations
- Return HTML fragments, not full pages
- Handle errors gracefully

### Security

- Sanitize all user input
- Use parameterized queries (built into db module)
- Validate form data server-side
- Don't expose sensitive data in templates

### Performance

- Static files are served efficiently
- Templates are cached in memory
- Database connections are pooled
- Minimize Python endpoint complexity

## Troubleshooting

### "No hrml.toml found"

Run `hrml check` to verify project structure.

### "Template not found"

Check that file exists in templates/ directory and path is correct.

### "Module not found" in Python

Ensure `__init__.py` files exist in endpoints/ and endpoints/api/.

### Changes not appearing

Restart the dev server or check that files are saved.

## Version History

- **v0.1.0**: Initial release
  - Template system
  - Python endpoints
  - Database integration
  - Static file serving
  - CLI tool

## Contributing

See CONTRIBUTING.md for guidelines on contributing to HRML.

## License

MIT License - See LICENSE file for details.
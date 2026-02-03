# HRML - Minimal Web Framework

A minimal, production-ready web framework combining Rust performance with Python flexibility. Create server-rendered web applications with dynamic interactivity using a simple HTML-first approach.

## Philosophy

HRML follows these principles:

- **Server-side first** - HTML rendered on the server for fast initial loads and SEO
- **Minimal JavaScript** - Only 3KB runtime for dynamic updates, no heavy frameworks
- **Simple templates** - HTML with clean, readable processing instructions
- **Python for logic** - Familiar scripting language for backend development
- **Rust for speed** - Fast HTTP handling and template rendering
- **Zero configuration** - Sensible defaults that just work

## Quick Start

### Installation

```bash
# Build from source
cargo build --release

# The binary will be at target/release/hrml
```

### Create a New Project

```bash
hrml new myapp
cd myapp
hrml dev
```

Visit http://localhost:8080

## Project Structure

HRML enforces a strict project structure for consistency:

```
myapp/
├── hrml.toml              # Configuration
├── templates/
│   ├── layouts/
│   │   └── base.hrml     # Base layout (required)
│   ├── components/       # Reusable components
│   │   └── nav.hrml
│   └── pages/
│       ├── index.hrml    # Home page (required)
│       └── about.hrml
├── endpoints/
│   └── api/              # Python endpoints
│       └── hello.py
└── static/
    ├── css/
    │   └── style.css
    └── js/
```

For complete documentation of the project structure and all behaviors, see [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md).

## CLI Commands

### `hrml new <name>`

Creates a new HRML project with the complete directory structure, configuration file, and sample files.

**Created structure:**
- `hrml.toml` - Configuration with sensible defaults
- `templates/layouts/base.hrml` - Base HTML layout
- `templates/components/nav.hrml` - Navigation component
- `templates/pages/index.hrml` - Home page
- `templates/pages/about.hrml` - About page
- `endpoints/api/hello.py` - Sample Python endpoint
- `static/css/style.css` - Default stylesheet
- `.gitignore` - Standard ignore patterns
- `README.md` - Project documentation

### `hrml dev [path]`

Runs the development server with the following behaviors:
- Serves the application on the configured host and port
- Watches for file changes
- Provides detailed error messages in the browser
- Logs all requests to stderr
- Uses the configuration from `hrml.toml`

**Default behavior:** Uses current directory if no path specified

### `hrml serve [path]`

Runs the production server:
- Optimized for production use
- Serves static files efficiently
- Handles concurrent requests
- Cleaner logging than dev mode

### `hrml check [path]`

Validates the project structure:
- Verifies `hrml.toml` exists and is valid
- Checks that required templates exist (base.hrml, index.hrml)
- Ensures template engine can be initialized
- Validates static directory exists
- Attempts to render index template
- Reports any warnings or errors

### `hrml version`

Displays the HRML version.

### `hrml help`

Shows command usage and examples.

## Template System

HRML uses a powerful but simple template system with the following directives:

### Layout System

**Base Layout** (`templates/layouts/base.hrml`):
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title><?get id="site_name"?></title>
    <link rel="stylesheet" href="/static/css/style.css">
    <script src="/hrml.js"></script>
</head>
<body>
    <?load file="components/nav.hrml"?>
    <main class="container">
        <?slot id="content"?>
    </main>
</body>
</html>
```

**Page Template** (`templates/pages/index.hrml`):
```html
<?load file="layouts/base.hrml"?>
<?block slot="content"?>
    <h1>Welcome</h1>
    <p>This is the home page.</p>
<?/block?>
```

### Template Directives

- `<?load file="path"?>` - Include another template
- `<?slot id="name"?>` - Define a content placeholder
- `<?block slot="name"?>...<?/block?>` - Fill a slot
- `<?set id="name"?>value<?/set?>` - Set a variable
- `<?get id="name"?>` - Get a variable value

## Python Endpoints

Create dynamic functionality with Python endpoints:

**File:** `endpoints/api/todos.py`
```python
def handler(req):
    """
    Handle API requests.
    
    req dict contains:
    - 'id': Resource ID (e.g., "123" for /api/todos/123)
    - 'action': Action name (e.g., "create" for /api/todos/create)
    - 'data': Form data as dictionary
    
    Returns: HTML string to be inserted into the page
    """
    import db
    import json
    
    action = req.get('action', '')
    data = req.get('data', {})
    
    if action == 'create':
        title = data.get('title', '')
        # Insert into database
        todo_id = db.table_insert('todos', json.dumps({'title': title, 'done': 0}))
        return f'<div id="todo-{todo_id}">{title}</div>'
    
    # Get all todos
    result = db.table_find_all('todos')
    todos = json.loads(result)
    
    html = ''
    for todo in todos:
        html += f'<div>{todo["title"]}</div>'
    return html
```

### Database API

HRML provides a built-in SQLite database with a simple API:

```python
import db
import json

# Create table
db.table_create('todos', 'id INTEGER PRIMARY KEY, title TEXT, done INTEGER')

# Insert
todo_id = db.table_insert('todos', json.dumps({'title': 'Buy milk', 'done': 0}))

# Find by ID
result = db.table_find('todos', todo_id)
item = json.loads(result)

# Get all
results = db.table_find_all('todos')
items = json.loads(results)

# Update
db.table_update('todos', todo_id, json.dumps({'done': 1}))

# Delete
db.table_delete('todos', todo_id)
```

## Client-Side Interactivity

HRML provides a 3KB JavaScript runtime at `/hrml.js` that enables dynamic updates:

### AJAX Buttons

```html
<button data-post="/api/counter/increment" 
        data-target="#counter-display"
        data-swap="innerHTML">
    Increment
</button>
```

**Behavior:** POSTs to the endpoint and replaces the target element's innerHTML with the response.

### AJAX Links

```html
<a href="/about" 
   data-get="/api/content/about"
   data-target="#content"
   data-swap="innerHTML">
   Load About
</a>
```

### Auto-Loading Content

```html
<div data-get="/api/todos" 
     data-trigger="load"
     data-swap="innerHTML">
    Loading...
</div>
```

**Behavior:** Automatically fetches content when the page loads.

### AJAX Forms

```html
<form data-post="/api/todos/create"
      data-target="#todo-list"
      data-swap="beforeend">
    <input name="title" required>
    <button type="submit">Add Todo</button>
</form>
```

**Behavior:** Submits form via AJAX, inserts response before the end of target element, resets form on success.

## Configuration

Configure your application in `hrml.toml`:

```toml
[project]
name = "my-app"
version = "1.0.0"

[server]
host = "127.0.0.1"
port = 8080

[paths]
templates = "templates"
endpoints = "endpoints"
static = "static"

[site]
name = "My Application"
description = "A great web app"
favicon = "/static/favicon.ico"
```

**Behavior:** If `hrml.toml` is missing, HRML uses sensible defaults:
- Host: 127.0.0.1
- Port: 8080
- All paths as shown above

## URL Routing

HRML automatically routes requests based on the URL structure:

### Pages

- `GET /` → Renders `templates/pages/index.hrml`
- `GET /about` → Renders `templates/pages/about.hrml`
- `GET /contact` → Renders `templates/pages/contact.hrml`

### API Endpoints

- `GET /api/todos` → Calls `endpoints/api/todos.py::handler()`
- `GET /api/todos/123` → Calls handler with `id="123"`
- `POST /api/todos/create` → Calls handler with `action="create"`
- `POST /api/todos/123/toggle` → Calls handler with `id="123"`, `action="toggle"`
- `DELETE /api/todos/123/delete` → Calls handler with `id="123"`, `action="delete"`

### Static Files

- `GET /static/css/style.css` → Serves `static/css/style.css`
- `GET /static/js/app.js` → Serves `static/js/app.js`

## Error Handling

### Development Mode

In development (`hrml dev`):
- Detailed error messages in browser
- Stack traces logged to stderr
- Template rendering errors show line numbers
- Python endpoint errors show full traceback

### Production Mode

In production (`hrml serve`):
- Generic error messages in browser
- Detailed errors logged to stderr only
- No stack traces exposed to clients

## Examples

### Todo List Application

**templates/pages/todos.hrml:**
```html
<?load file="layouts/base.hrml"?>
<?block slot="content"?>
    <h1>Todo List</h1>
    
    <form data-post="/api/todos/create"
          data-target="#todo-list"
          data-swap="beforeend">
        <input name="title" placeholder="New todo" required>
        <button type="submit">Add</button>
    </form>
    
    <div id="todo-list"
         data-get="/api/todos"
         data-trigger="load"
         data-swap="innerHTML">
        Loading...
    </div>
<?/block?>
```

**endpoints/api/todos.py:**
```python
def handler(req):
    import db
    import json
    
    action = req.get('action', '')
    data = req.get('data', {})
    
    if action == 'create':
        title = data.get('title', '')
        if title:
            todo_id = db.table_insert('todos', 
                json.dumps({'title': title, 'done': 0}))
            return f'<div id="todo-{todo_id}">{title}</div>'
        return ''
    
    # Return all todos
    result = db.table_find_all('todos')
    todos = json.loads(result)
    
    html = ''
    for todo in todos:
        checked = 'checked' if todo['done'] else ''
        html += f'''
        <div id="todo-{todo['id']}">
            <input type="checkbox" {checked} 
                   data-post="/api/todos/{todo['id']}/toggle"
                   data-target="#todo-{todo['id']}">
            {todo['title']}
        </div>
        '''
    return html
```

## Performance

HRML is designed for performance:

- **Templates** - Compiled and cached in memory
- **Static files** - Served efficiently with proper caching headers
- **Database** - Connection pooling and prepared statements
- **Minimal JavaScript** - 3KB runtime vs 100KB+ frameworks
- **Rust foundation** - Fast HTTP handling and concurrent request processing

## Security

HRML includes security best practices:

- **SQL Injection Protection** - All database queries use parameterized statements
- **XSS Prevention** - Template system escapes HTML by default
- **Static File Serving** - Proper MIME types and security headers
- **Input Validation** - Form data is properly parsed and validated

## Deployment

### Option 1: Binary + Project Files

```bash
# Build the binary
cargo build --release

# Copy binary and project to server
cp target/release/hrml /usr/local/bin/
cp -r myapp /var/www/

# Run on server
cd /var/www/myapp
hrml serve
```

### Option 2: Systemd Service

Create `/etc/systemd/system/hrml.service`:
```ini
[Unit]
Description=HRML Web Application
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=/var/www/myapp
ExecStart=/usr/local/bin/hrml serve
Restart=always

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
systemctl enable hrml
systemctl start hrml
```

## Development

### Running Tests

```bash
cd /root/hrml/example
hrml check          # Validate project
hrml dev            # Start dev server
```

### Project Validation

Before deploying, always validate:

```bash
hrml check
```

This ensures:
- All required files exist
- Templates are valid
- Configuration is correct
- Static directories exist

## Troubleshooting

### "No hrml.toml found"

Run `hrml check` to verify you're in the correct directory.

### "Template not found"

Check that the template file exists in the correct subdirectory of `templates/`.

### "Module not found" in Python

Ensure `__init__.py` files exist in `endpoints/` and `endpoints/api/`.

### Changes not appearing

- Restart the dev server: `hrml dev`
- Check browser cache (Ctrl+Shift+R to hard reload)
- Verify file was saved

### Port already in use

Change the port in `hrml.toml`:
```toml
[server]
port = 3000  # Use a different port
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - See [LICENSE](LICENSE) file for details.

## Documentation

- [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) - Complete project structure documentation
- [CHANGELOG.md](CHANGELOG.md) - Version history and changes
- [API.md](API.md) - Detailed API documentation (coming soon)
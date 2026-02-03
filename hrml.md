# HRML Framework Specification

**Version:** 0.1.0  
**Status:** Draft

---

## 1. Overview

HRML is a full-stack web framework combining:
- **Rust** – HTTP server, routing, template engine, and Python orchestration
- **Python** – backend scripts for business logic, DB access, and data processing
- **HRML templates** – HTML-first markup with data binding and composition
- **HTMX** – declarative client interactivity without JavaScript

The framework prioritizes **simplicity**, **composability**, and **clear separation** between presentation (HRML), logic (Python), and infrastructure (Rust).

---

## 2. Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Browser                              │
│   HTMX requests ←──────────────────────────────────────────┐│
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Rust Runtime                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ HTTP Server │→ │   Router    │→ │  Template Engine    │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│                          │                                  │
│                          ▼                                  │
│                  ┌─────────────┐                            │
│                  │ Python Host │                            │
│                  │  (PyO3)     │                            │
│                  └─────────────┘                            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Python Scripts                           │
│   endpoints/*.py   │   lib/*.py   │   db.py                 │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Project Structure

```
project/
├── hrml.toml              # project config
├── static/                # served as-is
│   ├── css/
│   ├── js/
│   └── assets/
├── templates/             # .hrml files
│   ├── layouts/
│   │   └── base.hrml
│   ├── components/
│   │   ├── nav.hrml
│   │   └── card.hrml
│   └── pages/
│       ├── index.hrml
│       └── dashboard.hrml
├── endpoints/             # Python scripts mapped to routes
│   ├── auth.py
│   ├── users.py
│   └── api/
│       └── data.py
└── lib/                   # shared Python modules
    ├── db.py
    └── utils.py
```

---

## 4. Configuration (`hrml.toml`)

```toml
[server]
host = "127.0.0.1"
port = 8080
workers = 4

[paths]
templates = "templates"
endpoints = "endpoints"
static = "static"
lib = "lib"

[python]
venv = ".venv"              # optional virtualenv path
modules = ["lib"]           # auto-import paths

[security]
allowed_origins = ["*"]
escape_html = true          # default escape policy
```

---

## 5. Routing

Routes are defined by **file structure** in `endpoints/` and **page files** in `templates/pages/`.

### 5.1 Page Routes (GET)
Templates in `templates/pages/` are automatically served:
- `pages/index.hrml` → `GET /`
- `pages/dashboard.hrml` → `GET /dashboard`
- `pages/users/profile.hrml` → `GET /users/profile`

### 5.2 Endpoint Routes (Python)
Python files in `endpoints/` expose functions as routes:

```python
# endpoints/auth.py

def login(req):
    """POST /auth/login"""
    username = req.form["username"]
    password = req.form["password"]
    # ... validate ...
    return {"ok": True, "user": {"name": username}}

def logout(req):
    """POST /auth/logout"""
    req.session.clear()
    return {"ok": True}
```

Route mapping:
- `endpoints/auth.py::login` → `POST /auth/login`
- `endpoints/api/data.py::fetch` → `POST /api/data/fetch`

### 5.3 Route Attributes
Use decorators for fine-grained control:

```python
from hrml import route, template

@route("/login", methods=["GET", "POST"])
def login(req):
    if req.method == "GET":
        return template("pages/login.hrml")
    # POST logic...
    return {"ok": True}

@route("/users/<int:id>")
def get_user(req, id):
    return db.get_user(id)
```

---

## 6. HRML Template Syntax

### 6.1 Processing Instructions (PIs)

| PI | Purpose |
|----|---------|
| `<?load?>` | Include another template |
| `<?slot?>` | Define insertion point in layouts |
| `<?block?>` | Fill a slot from child template |
| `<?set?>` | Bind data to a name |
| `<?get?>` | Retrieve bound data |
| `<?if?>` `<?else?>` | Conditional rendering |
| `<?for?>` | Iteration |
| `<?call?>` | Invoke Python endpoint |
| `<?style?>` | Scoped CSS |
| `<?script?>` | Inline JS with data binding |

All PIs close with `</?name>`.

---

### 6.2 `<?load file="..."?>`
Include and inline another template.

```html
<?load file="components/nav.hrml"?>
```

---

### 6.3 `<?slot?>` and `<?block?>`
Layouts define slots; pages fill them.

**layouts/base.hrml:**
```html
<!DOCTYPE html>
<html>
<head>
  <title><?slot id="title">Default</?slot></title>
  <?slot id="head"?><?/?slot?>
</head>
<body>
  <?load file="components/nav.hrml"?>
  <main>
    <?slot id="content"?><?/?slot?>
  </main>
</body>
</html>
```

**pages/index.hrml:**
```html
<?load file="layouts/base.hrml"?>

<?block slot="title">Home</?block?>
<?block slot="content">
  <h1>Welcome</h1>
</?block?>
```

---

### 6.4 `<?set?>` and `<?get?>`
Data binding within templates.

```html
<?set id="user">
  <name>Alice</name>
  <role>admin</role>
</?set?>

<p>Hello, <?get id="user.name"?>!</p>
<p>Role: <?get id="user.role"?></p>
```

Shorthand dot notation: `user.name` is equivalent to `user#name`.

---

### 6.5 `<?if?>` and `<?else?>`
Conditional rendering with expressions.

```html
<?if cond="user.role == 'admin'"?>
  <button>Delete</button>
<?else?>
  <span>View only</span>
</?else?>
</?if?>
```

**Operators:** `==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`, `!`, `in`, `not in`

**Truthy/Falsy:** empty string, `null`, `0`, `false`, empty list → falsy.

---

### 6.6 `<?for?>`
Iteration over lists.

```html
<?for item in="items" index="i"?>
  <li data-index="<?get id="i"?>"><?get id="item.name"?></li>
</?for?>
```

**Attributes:**
- `item` (required): loop variable name
- `in` (required): iterable expression
- `index` (optional): index variable name

---

### 6.7 `<?call?>`
Fetch data from a Python endpoint at render time.

```html
<?call endpoint="/api/users/list" as="users"?>
  <error>
    <p>Failed to load users</p>
  </error>
</?call?>

<?for user in="users"?>
  <div class="card"><?get id="user.name"?></div>
</?for?>
```

**Attributes:**
- `endpoint` (required): route path
- `as` (required): variable name for result
- `method` (optional, default: GET)
- `cache` (optional): cache duration in seconds

**Error handling:** `<error>` child is rendered if the call fails.

---

### 6.8 `<?style?>` – Scoped CSS
Component-scoped styles using automatic class prefixing.

```html
<?style scoped?>
  .card {
    padding: 1rem;
    border: 1px solid #ccc;
  }
  .card:hover {
    box-shadow: 0 2px 8px rgba(0,0,0,0.1);
  }
</?style?>

<div class="card">
  Content here
</div>
```

**Output:**
```html
<style>
  .card-x7f3a { padding: 1rem; border: 1px solid #ccc; }
  .card-x7f3a:hover { box-shadow: 0 2px 8px rgba(0,0,0,0.1); }
</style>
<div class="card-x7f3a">Content here</div>
```

**Attributes:**
- `scoped` (flag): enable class hashing
- `global` (flag): emit as global styles (no transformation)

---

### 6.9 `<?script?>` – Data-bound JavaScript
Inline scripts with access to template data.

```html
<?set id="config">
  <apiUrl>/api</apiUrl>
  <debug>true</debug>
</?set?>

<?script?>
  const config = <?json id="config"?>;
  console.log("API:", config.apiUrl);
</?script?>
```

The `<?json?>` PI inside `<?script?>` serializes template data to JSON.

---

## 7. HTMX Integration

HRML is designed for HTMX-driven interactivity. No `<?hx-on?>` server-side blocks—instead, HTMX requests hit Python endpoints that return rendered partials.

### 7.1 Pattern: Interactive Component

```html
<!-- components/counter.hrml -->
<?set id="count">0</?set?>

<div id="counter" class="counter">
  <span><?get id="count"?></span>
  <button hx-post="/api/counter/increment"
          hx-target="#counter"
          hx-swap="outerHTML">
    +1
  </button>
</div>
```

```python
# endpoints/api/counter.py
from hrml import template

count = 0  # In real app, use session/db

def increment(req):
    global count
    count += 1
    return template("components/counter.hrml", {"count": count})
```

### 7.2 Pattern: Form Submission

```html
<form hx-post="/auth/login"
      hx-target="#result"
      hx-swap="innerHTML">
  <input name="username" required>
  <input name="password" type="password" required>
  <button type="submit">Login</button>
</form>
<div id="result"></div>
```

```python
# endpoints/auth.py
from hrml import template, html

def login(req):
    user = authenticate(req.form["username"], req.form["password"])
    if user:
        return template("partials/login_success.hrml", {"user": user})
    return html('<p class="error">Invalid credentials</p>')
```

### 7.3 HTMX Response Helpers

Python endpoints can use helpers:

```python
from hrml import (
    template,      # render a template
    html,          # raw HTML string
    redirect,      # HX-Redirect header
    refresh,       # HX-Refresh header
    trigger,       # HX-Trigger header
    swap,          # override hx-swap
)

def logout(req):
    req.session.clear()
    return redirect("/") | trigger("logged-out")
```

---

## 8. Python Endpoint API

### 8.1 Request Object

```python
def handler(req):
    req.method          # "GET", "POST", etc.
    req.path            # "/users/123"
    req.params          # {"id": "123"} from route
    req.query           # {"page": "1"} from ?page=1
    req.form            # POST form data
    req.json            # parsed JSON body
    req.headers         # dict of headers
    req.cookies         # dict of cookies
    req.session         # session dict (mutable)
    req.files           # uploaded files
```

### 8.2 Response Types

```python
# Dict → JSON response
return {"ok": True, "data": [...]}

# String → HTML response
return "<p>Hello</p>"

# Template render
return template("path.hrml", {"var": value})

# Explicit response
from hrml import Response
return Response(
    body="...",
    status=201,
    headers={"X-Custom": "value"},
    cookies={"token": "abc"}
)
```

### 8.3 Database Access

```python
# lib/db.py
from hrml import Database

db = Database("sqlite:///app.db")
# or: Database("postgresql://user:pass@host/db")

# In endpoint:
from lib.db import db

def list_users(req):
    users = db.query("SELECT * FROM users WHERE active = ?", [True])
    return {"users": users}

def create_user(req):
    db.execute(
        "INSERT INTO users (name, email) VALUES (?, ?)",
        [req.form["name"], req.form["email"]]
    )
    return {"ok": True}
```

---

## 9. Expressions

### 9.1 Grammar

```
expr     = or_expr
or_expr  = and_expr ("||" and_expr)*
and_expr = cmp_expr ("&&" cmp_expr)*
cmp_expr = add_expr (("==" | "!=" | "<" | ">" | "<=" | ">=" | "in") add_expr)?
add_expr = mul_expr (("+" | "-") mul_expr)*
mul_expr = unary (("*" | "/" | "%") unary)*
unary    = "!" unary | primary
primary  = literal | variable | "(" expr ")" | fn_call
variable = ident ("." ident)*
fn_call  = ident "(" (expr ("," expr)*)? ")"
literal  = string | number | "true" | "false" | "null"
```

### 9.2 Built-in Functions

| Function | Description |
|----------|-------------|
| `len(x)` | Length of string/list |
| `upper(s)` | Uppercase string |
| `lower(s)` | Lowercase string |
| `trim(s)` | Strip whitespace |
| `split(s, d)` | Split string by delimiter |
| `join(list, d)` | Join list with delimiter |
| `default(x, d)` | Return `d` if `x` is null/empty |
| `date(ts, fmt)` | Format timestamp |
| `json(x)` | Serialize to JSON string |
| `escape(s)` | HTML escape |
| `safe(s)` | Mark as safe (no escape) |

---

## 10. Static Assets & Bundling

### 10.1 Static Files
Files in `static/` are served directly:
- `static/css/main.css` → `/css/main.css`
- `static/js/app.js` → `/js/app.js`

### 10.2 Asset Helpers

```html
<link rel="stylesheet" href="<?asset path='css/main.css'?>">
<script src="<?asset path='js/app.js'?>"></script>
<img src="<?asset path='assets/logo.png'?>">
```

In production, `<?asset?>` appends content hash for cache busting:
`/css/main.css?v=a3f2b1c`

### 10.3 CSS Variables from Template

```html
<?style?>
  :root {
    --primary: <?get id="theme.primary" default="#3b82f6"?>;
    --bg: <?get id="theme.background" default="#ffffff"?>;
  }
</?style?>
```

---

## 11. Components

### 11.1 Component Definition

```html
<!-- components/card.hrml -->
<?slot id="title"?>Untitled</?slot?>
<?slot id="body"?></?slot?>
<?slot id="footer"?></?slot?>

<?style scoped?>
  .card { border: 1px solid #e5e7eb; border-radius: 8px; }
  .card-title { font-weight: bold; padding: 1rem; }
  .card-body { padding: 1rem; }
  .card-footer { padding: 1rem; border-top: 1px solid #e5e7eb; }
</?style?>

<div class="card">
  <div class="card-title"><?slot id="title"?></div>
  <div class="card-body"><?slot id="body"?></div>
  <?if cond="has_slot('footer')"?>
    <div class="card-footer"><?slot id="footer"?></div>
  </?if?>
</div>
```

### 11.2 Component Usage

```html
<?load file="components/card.hrml" as="Card"?>

<?Card?>
  <?block slot="title"?>User Profile</?block?>
  <?block slot="body"?>
    <p>Name: <?get id="user.name"?></p>
  </?block?>
</?Card?>
```

---

## 12. Error Handling

### 12.1 Template Errors
- Missing variables render as empty string (with warning in dev mode)
- Parse errors halt compilation with file:line info

### 12.2 Endpoint Errors

```python
from hrml import HttpError, template

def get_user(req, id):
    user = db.get_user(id)
    if not user:
        raise HttpError(404, "User not found")
    return template("pages/user.hrml", {"user": user})
```

### 12.3 Error Pages
Define custom error templates:
- `templates/errors/404.hrml`
- `templates/errors/500.hrml`

---

## 13. Development Server

```bash
hrml dev                    # start dev server with hot reload
hrml dev --port 3000        # custom port
hrml build                  # production build
hrml serve                  # serve production build
```

### 13.1 Hot Reload
- Template changes: instant reload
- Python changes: process restart
- Static assets: browser refresh

### 13.2 Dev Tools
- Error overlay in browser
- Request logging
- Template inspector (shows data context)

---

## 14. Production Build

```bash
hrml build --release
```

**Output:**
```
dist/
├── server              # compiled Rust binary
├── templates/          # pre-parsed templates
├── static/             # optimized assets
│   ├── css/
│   │   └── main.a3f2b1.css
│   └── js/
│       └── app.c4d5e6.js
└── endpoints/          # Python bytecode (.pyc)
```

---

## 15. Examples

### 15.1 Todo App

**templates/pages/todos.hrml:**
```html
<?load file="layouts/base.hrml"?>

<?block slot="title"?>Todos</?block?>
<?block slot="content"?>
  <?call endpoint="/api/todos" as="todos"?>
    <error><p>Failed to load</p></error>
  </?call?>

  <div id="todo-list">
    <?for todo in="todos"?>
      <?load file="components/todo-item.hrml"?>
    </?for?>
  </div>

  <form hx-post="/api/todos/create"
        hx-target="#todo-list"
        hx-swap="beforeend">
    <input name="title" placeholder="New todo..." required>
    <button type="submit">Add</button>
  </form>
</?block?>
```

**endpoints/api/todos.py:**
```python
from hrml import template
from lib.db import db

def index(req):
    """GET /api/todos"""
    return db.query("SELECT * FROM todos ORDER BY created_at DESC")

def create(req):
    """POST /api/todos/create"""
    db.execute("INSERT INTO todos (title) VALUES (?)", [req.form["title"]])
    todo = db.query_one("SELECT * FROM todos ORDER BY id DESC LIMIT 1")
    return template("components/todo-item.hrml", {"todo": todo})

def toggle(req, id):
    """POST /api/todos/<id>/toggle"""
    db.execute("UPDATE todos SET done = NOT done WHERE id = ?", [id])
    todo = db.query_one("SELECT * FROM todos WHERE id = ?", [id])
    return template("components/todo-item.hrml", {"todo": todo})
```

### 15.2 Dashboard with Live Data

```html
<?load file="layouts/base.hrml"?>

<?block slot="head"?>
  <?style scoped?>
    .stat-card { display: flex; flex-direction: column; padding: 1.5rem; }
    .stat-value { font-size: 2rem; font-weight: bold; }
    .stat-label { color: #6b7280; }
  </?style?>
</?block?>

<?block slot="content"?>
  <div class="grid grid-cols-3 gap-4"
       hx-get="/api/stats"
       hx-trigger="load, every 30s"
       hx-swap="innerHTML">
    <!-- Stats loaded via HTMX -->
  </div>
</?block?>
```

---

## 16. Security

### 16.1 Auto-Escaping
All `<?get?>` output is HTML-escaped by default. Use `safe()` to bypass:

```html
<?get id="user_html" | safe?>
```

### 16.2 CSRF Protection
Forms automatically include CSRF token:

```html
<form method="POST">
  <!-- <?csrf?> auto-injected -->
  ...
</form>
```

### 16.3 Content Security Policy
Configure in `hrml.toml`:

```toml
[security.csp]
default-src = ["'self'"]
script-src = ["'self'", "'unsafe-inline'"]
style-src = ["'self'", "'unsafe-inline'"]
```

---

## 17. Non-Goals

- **SPA routing** – Use HTMX for partial updates, not client-side routing
- **Complex state management** – Server is the source of truth
- **Build-time JS bundling** – Use external tools if needed
- **ORM** – Simple query interface; use SQLAlchemy in `lib/` if needed

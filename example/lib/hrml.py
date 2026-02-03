"""
HRML Python helpers - Auto-imported into endpoints
"""

class Table:
    """Database table interface"""
    def __init__(self, name):
        self._name = name
    
    def create(self, schema):
        """Create table with schema"""
        import db
        return db.table_create(self._name, schema)
    
    def insert(self, **data):
        """Insert row and return ID"""
        import db
        import json
        return db.table_insert(self._name, json.dumps(data))
    
    def find(self, id):
        """Find row by ID"""
        import db
        import json
        result = db.table_find(self._name, id)
        return json.loads(result) if result else None
    
    def all(self):
        """Get all rows"""
        import db
        import json
        results = db.table_find_all(self._name)
        return json.loads(results) if results else []
    
    def update(self, id, **data):
        """Update row"""
        import db
        import json
        return db.table_update(self._name, id, json.dumps(data))
    
    def delete(self, id):
        """Delete row"""
        import db
        return db.table_delete(self._name, id)

class Html:
    """HTML builder for clean response construction"""
    def __init__(self):
        self._parts = []
    
    def div(self, content, class_name=None, id=None):
        """Add a div"""
        attrs = []
        if class_name:
            attrs.append(f'class="{class_name}"')
        if id:
            attrs.append(f'id="{id}"')
        attr_str = " " + " ".join(attrs) if attrs else ""
        self._parts.append(f'<div{attr_str}>{content}</div>')
        return self
    
    def p(self, content):
        """Add a paragraph"""
        self._parts.append(f'<p>{content}</p>')
        return self
    
    def h1(self, content):
        self._parts.append(f'<h1>{content}</h1>')
        return self
    
    def h2(self, content):
        self._parts.append(f'<h2>{content}</h2>')
        return self
    
    def h3(self, content):
        self._parts.append(f'<h3>{content}</h3>')
        return self
    
    def span(self, content):
        self._parts.append(f'<span>{content}</span>')
        return self
    
    def button(self, text, post=None, target=None, classes="btn btn-primary"):
        """Add button with data-post"""
        attrs = [f'class="{classes}"']
        if post:
            attrs.append(f'data-post="{post}"')
        if target:
            attrs.append(f'data-target="{target}"')
            attrs.append('data-swap="innerHTML"')
        self._parts.append(f'<button {" ".join(attrs)}>{text}</button>')
        return self
    
    def checkbox(self, name, checked=False, post=None, target=None):
        """Add checkbox with data-post"""
        attrs = [f'type="checkbox"', f'name="{name}"']
        if checked:
            attrs.append('checked')
        if post:
            attrs.append(f'data-post="{post}"')
        if target:
            attrs.append(f'data-target="{target}"')
            attrs.append('data-swap="outerHTML"')
        self._parts.append(f'<input {" ".join(attrs)}>')
        return self
    
    def input(self, name, type="text", placeholder=""):
        """Add input field"""
        self._parts.append(
            f'<input type="{type}" name="{name}" placeholder="{placeholder}">'
        )
        return self
    
    def link(self, href, text):
        """Add link"""
        self._parts.append(f'<a href="{href}">{text}</a>')
        return self
    
    def raw(self, html):
        """Add raw HTML"""
        self._parts.append(html)
        return self
    
    def build(self):
        """Build final HTML string"""
        return '\n'.join(self._parts)
    
    def __str__(self):
        return self.build()

def table(name):
    """Get table interface"""
    return Table(name)

def html():
    """Create HTML builder"""
    return Html()

def escape(text):
    """HTML escape text"""
    import html as html_mod
    return html_mod.escape_html(text)

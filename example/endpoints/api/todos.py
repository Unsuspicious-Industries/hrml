"""
Todo endpoints - demonstrates full CRUD with typed database interface
"""

import sys
sys.path.insert(0, 'lib')
from hrml import table, html

# Initialize todos table
todos = table('todos')
todos.create('''
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    done INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
''')

def handler(req):
    """Handle todos endpoint with different actions"""
    action = req.get('action', '')
    data = req.get('data', {})
    todo_id = req.get('id', '')
    
    if action == 'create':
        # POST /api/todos/create - Create new todo
        title = data.get('title', '').strip()
        if not title:
            return ''  # Return empty string to avoid duplicate error messages
        
        new_id = todos.insert(title=title, done=0)
        todo = todos.find(new_id)
        
        return (html()
            .div(
                (html()
                    .checkbox("done", 
                             checked=False,
                             post=f"/api/todos/{new_id}/toggle",
                             target=f"#todo-{new_id}")
                    .span(title)
                    .raw(f'''<button class="btn-delete" 
                             data-delete="/api/todos/{new_id}/delete"
                             data-target="#todo-{new_id}"
                             data-swap="outerHTML">×</button>''')
                    .build()
                ),
                class_name="todo-item",
                id=f"todo-{new_id}"
            )
            .build()
        )
    
    elif action == 'toggle':
        # POST /api/todos/{id}/toggle - Toggle todo done status
        if not todo_id:
            return ''
        
        todo = todos.find(int(todo_id))
        if not todo:
            return ''
        
        # Toggle done status
        new_done = 0 if todo['done'] else 1
        todos.update(int(todo_id), done=new_done)
        
        # Return updated todo item
        done_class = "done" if new_done else ""
        return (html()
            .div(
                (html()
                    .checkbox("done", 
                             checked=bool(new_done),
                             post=f"/api/todos/{todo_id}/toggle",
                             target=f"#todo-{todo_id}")
                    .span(todo['title'])
                    .raw(f'''<button class="btn-delete" 
                             data-delete="/api/todos/{todo_id}/delete"
                             data-target="#todo-{todo_id}"
                             data-swap="outerHTML">×</button>''')
                    .build()
                ),
                class_name=f"todo-item {done_class}",
                id=f"todo-{todo_id}"
            )
            .build()
        )
    
    elif action == 'delete':
        # DELETE /api/todos/{id}/delete - Delete todo
        if not todo_id:
            return ''
        
        todos.delete(int(todo_id))
        return ''  # Return empty to remove element
    
    # Default: GET /api/todos - List all todos
    all_todos = todos.all()
    
    if not all_todos:
        return '<p class="empty">No todos yet. Add one above!</p>'
    
    builder = html()
    for todo in all_todos:
        done_class = "done" if todo['done'] else ""
        builder.div(
            (html()
                .checkbox("done", 
                         checked=bool(todo['done']),
                         post=f"/api/todos/{todo['id']}/toggle",
                         target=f"#todo-{todo['id']}")
                .span(todo['title'])
                .raw(f'''<button class="btn-delete" 
                         data-delete="/api/todos/{todo['id']}/delete"
                         data-target="#todo-{todo['id']}"
                         data-swap="outerHTML">×</button>''')
                .build()
            ),
            class_name=f"todo-item {done_class}",
            id=f"todo-{todo['id']}"
        )
    
    return builder.build()

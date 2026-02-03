"""
Blog endpoints - full blog system with posts
"""

import sys
sys.path.insert(0, 'lib')
from hrml import table, html

# Initialize blog posts table
posts = table('blog_posts')
posts.create('''
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    author TEXT DEFAULT 'Admin',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
''')

def handler(req):
    """Handle blog endpoint with different actions"""
    action = req.get('action', '')
    data = req.get('data', {})
    
    if action == 'create':
        # POST /api/blog/create - Create new blog post
        title = data.get('title', '').strip()
        content = data.get('content', '').strip()
        author = data.get('author', 'Admin').strip()
        
        if not title or not content:
            return ''
        
        post_id = posts.insert(
            title=title,
            content=content,
            author=author
        )
        
        return render_post_item(posts.find(post_id))
    
    elif action.startswith('delete') or 'delete' in action:
        # Handle delete with various action formats
        # Extract ID from action if present (e.g., "1/delete")
        parts = action.split('/')
        post_id = None
        if len(parts) > 1:
            try:
                post_id = int(parts[0])
            except:
                pass
        
        if not post_id:
            post_id = data.get('id')
        
        if post_id:
            posts.delete(int(post_id))
        return ''
    
    elif action.startswith('edit') or 'edit' in action:
        # GET /api/blog/{id}/edit - Return edit form
        parts = action.split('/')
        post_id = None
        if len(parts) > 1:
            try:
                post_id = int(parts[0])
            except:
                pass
        
        if not post_id:
            post_id = data.get('id')
        
        if not post_id:
            return '<div class="error">Invalid post ID</div>'
        
        post = posts.find(int(post_id))
        if not post:
            return '<div class="error">Post not found</div>'
        
        return render_edit_form(post)
    
    elif action.startswith('update') or 'update' in action:
        # POST /api/blog/{id}/update
        parts = action.split('/')
        post_id = None
        if len(parts) > 1:
            try:
                post_id = int(parts[0])
            except:
                pass
        
        if not post_id:
            post_id = data.get('id')
            
        title = data.get('title', '').strip()
        content = data.get('content', '').strip()
        author = data.get('author', '').strip()
        
        if not post_id or not title or not content:
            return '<div class="error">Invalid update data</div>'
        
        posts.update(int(post_id), title=title, content=content, author=author)
        return render_post_item(posts.find(int(post_id)))
    
    # Default: GET /api/blog - List all blog posts
    all_posts = posts.all()
    
    if not all_posts:
        return render_empty_state()
    
    # Sort by created_at descending (newest first)
    all_posts.sort(key=lambda p: p.get('created_at', ''), reverse=True)
    
    builder = html()
    for post in all_posts:
        builder.raw(render_post_item(post))
    
    return builder.build()


def render_post_item(post):
    """Render a single blog post"""
    post_id = post['id']
    title = post['title']
    content = post['content']
    author = post.get('author', 'Admin')
    created = post.get('created_at', '')
    
    # Format date if available
    date_display = created.split()[0] if created else 'Unknown date'
    
    # Escape HTML in content
    content_escaped = content.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;')
    
    # Truncate content for preview (show first 150 chars)
    preview = content_escaped[:150] + '...' if len(content_escaped) > 150 else content_escaped
    
    return f'''
    <article class="blog-post" id="post-{post_id}">
        <div class="post-header">
            <h2 class="post-title">{title}</h2>
            <div class="post-meta">
                <span class="author">By {author}</span>
                <span class="date">{date_display}</span>
            </div>
        </div>
        <div class="post-preview" id="preview-{post_id}">
            <p>{preview}</p>
        </div>
        <div class="post-full" id="full-{post_id}" style="display:none;">
            <p style="white-space: pre-wrap;">{content_escaped}</p>
        </div>
        <div class="post-actions">
            <button class="btn btn-small" onclick="toggleContent('{post_id}')">Read More</button>
            <button class="btn btn-small btn-edit" 
                    data-get="/api/blog/{post_id}/edit"
                    data-target="#post-{post_id}"
                    data-swap="outerHTML">Edit</button>
            <button class="btn btn-small btn-danger" 
                    data-post="/api/blog/{post_id}/delete"
                    data-target="#post-{post_id}"
                    data-swap="outerHTML">Delete</button>
        </div>
    </article>
    '''


def render_edit_form(post):
    """Render edit form for a blog post"""
    post_id = post['id']
    title = post['title'].replace('"', '&quot;')
    content = post['content']
    author = post.get('author', 'Admin').replace('"', '&quot;')
    
    return f'''
    <article class="blog-post-edit" id="post-{post_id}">
        <form data-post="/api/blog/{post_id}/update" 
              data-target="#post-{post_id}" 
              data-swap="outerHTML">
            <div class="form-group">
                <label>Title</label>
                <input type="text" name="title" value="{title}" required class="form-input">
            </div>
            <div class="form-group">
                <label>Author</label>
                <input type="text" name="author" value="{author}" class="form-input">
            </div>
            <div class="form-group">
                <label>Content</label>
                <textarea name="content" required class="form-textarea" rows="8">{content}</textarea>
            </div>
            <input type="hidden" name="id" value="{post_id}">
            <div class="form-actions">
                <button type="submit" class="btn btn-primary">Save Changes</button>
                <button type="button" class="btn btn-secondary" 
                        data-get="/api/blog"
                        data-target="#posts-list"
                        data-swap="innerHTML">Cancel</button>
            </div>
        </form>
    </article>
    '''


def render_empty_state():
    """Render empty state when no posts exist"""
    return '''
    <div class="empty-state">
        <h3>📝 No blog posts yet</h3>
        <p>Create your first post using the form above!</p>
    </div>
    '''

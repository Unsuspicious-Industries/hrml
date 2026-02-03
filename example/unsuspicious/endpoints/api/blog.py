def handler(req):
    import db
    import json
    
    action = req.get('action', '')
    data = req.get('data', {})
    
    if action == 'create':
        title = data.get('title', '')
        content = data.get('content', '')
        author = data.get('author', '')
        
        blog_id = db.table_insert('blog', json.dumps({
            'title': title,
            'content': content,
            'author': author,
            'published': 1,
            'created_at': 'now'
        }))
        
        return f'''<article class="blog-post" id="post-{blog_id}">
            <div class="post-header">
                <h2 class="post-title">{title}</h2>
                <div class="post-meta">
                    <span class="author">By {author}</span>
                </div>
            </div>
            <div class="post-content">{content}</div>
        </article>'''
    
    elif action == 'list':
        results = db.table_find_all('blog')
        posts = json.loads(results) if results else '[]'
        
        html = ''
        for post in posts:
            html += f'''
                <article class="blog-post" id="post-{post['id']}">
                    <div class="post-header">
                        <h2 class="post-title">{post['title']}</h2>
                        <div class="post-meta">
                            <span class="author">By {post.get('author', 'Unknown')}</span>
                        </div>
                    </div>
                    <div class="post-content">{post.get('content', '')}</div>
                </article>'''
        
        if not html:
            html = '<div class="empty-state"><h3>No blog posts yet</h3><p>Create your first post using the form above!</p></div>'
        
        return html
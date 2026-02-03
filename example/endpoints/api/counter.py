"""
Counter endpoint - demonstrates HTMX interactivity with database
"""

import sys
sys.path.insert(0, 'lib')
from hrml import table, html

# Initialize counter table
counters = table('counters')
counters.create('id INTEGER PRIMARY KEY, value INTEGER NOT NULL')

# Ensure we have a counter row
if not counters.all():
    counters.insert(id=1, value=0)

def handler(req):
    """
    POST /api/counter/increment
    Increments counter in database and returns updated HTML
    """
    # Get current value
    counter = counters.find(1)
    current = counter['value'] if counter else 0
    
    # Increment
    new_value = current + 1
    counters.update(1, value=new_value)
    
    # Build response using HTML builder
    return (html()
        .div(str(new_value), class_name="count-value")
        .button("Increment +1", 
                post="/api/counter/increment",
                target="#counter-display")
        .build()
    )

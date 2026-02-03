#!/usr/bin/env python3
import sys
import os

# Add current directory to Python path
sys.path.insert(0, '.')

print("Current working directory:", os.getcwd())
print("Python path:", sys.path[:3])  # Show first 3 entries

try:
    print("Testing import of endpoints.api.counter...")
    import endpoints.api.counter
    print("SUCCESS: endpoints.api.counter imported")
    
    print("Testing module attributes...")
    print("handler function:", hasattr(endpoints.api.counter, 'handler'))
    
except Exception as e:
    print("ERROR:", e)
    import traceback
    traceback.print_exc()
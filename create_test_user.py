#!/usr/bin/env python3
"""Create a test user in Cortex SurrealDB"""

import hashlib
import uuid
from datetime import datetime
import json

# Use bcrypt to hash password
import bcrypt

def create_user_data(email: str, password: str, roles: list[str] = None):
    """Create user data structure for SurrealDB"""
    if roles is None:
        roles = ["admin"]

    # Hash password with bcrypt (cost=12, same as Rust DEFAULT_COST)
    password_hash = bcrypt.hashpw(password.encode('utf-8'), bcrypt.gensalt(rounds=12)).decode('utf-8')

    user_id = str(uuid.uuid4())
    now = datetime.utcnow().isoformat() + "Z"

    user = {
        "id": user_id,
        "email": email,
        "password_hash": password_hash,
        "roles": roles,
        "created_at": now,
        "updated_at": now
    }

    return user, user_id

def main():
    email = "test@example.com"
    password = "password123"

    user, user_id = create_user_data(email, password, ["admin"])

    print("User data created:")
    print(json.dumps(user, indent=2))
    print()
    print(f"To insert into SurrealDB, run:")
    print(f"surreal sql --endpoint http://localhost:8000 --namespace cortex --database cortex --username root --password root")
    print()
    print("Then execute:")
    print(f"CREATE users:{user_id} CONTENT {json.dumps(user)};")
    print()
    print("Or use curl:")
    curl_data = json.dumps(user)
    print(f'''curl -X POST http://localhost:8000/sql \\
  -H "Accept: application/json" \\
  -H "NS: cortex" \\
  -H "DB: cortex" \\
  -u "root:root" \\
  -d "CREATE users:{user_id} CONTENT {curl_data};"''')

if __name__ == "__main__":
    main()

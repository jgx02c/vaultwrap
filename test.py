#!/usr/bin/env python3
# Simple test script to verify environment variables are working

import os
import sys

def main():
    print("=== Environment Variables ===")
    
    # Check for expected environment variables from vault
    env_vars = ['DATABASE_URL', 'API_KEY', 'SECRET_KEY', 'REDIS_URL', 'ENVIRONMENT']
    
    for var in env_vars:
        value = os.environ.get(var)
        if value:
            # Show only first few characters of sensitive values
            if var in ['API_KEY', 'SECRET_KEY']:
                displayed_value = value[:8] + '...' if len(value) > 8 else value
            else:
                displayed_value = value
            print(f"{var}: {displayed_value}")
        else:
            print(f"{var}: Not found")
    
    # Print all environment variables with 'VAULT' in the name
    print("\n=== All VAULT_ Variables ===")
    for key, value in os.environ.items():
        if 'VAULT' in key:
            print(f"{key}: {value}")

if __name__ == "__main__":
    main()

import requests
import json
import time
import sys

url = "http://127.0.0.1:15721"
payload = {
    "jsonrpc": "2.0",
    "method": "bevy/list",
    "id": 1,
    "params": {}
}

print("Waiting for Bevy Remote to come online...")
start_time = time.time()
while time.time() - start_time < 300: # Wait up to 5 minutes for compilation
    try:
        response = requests.post(url, json=payload, timeout=1)
        if response.status_code == 200:
            print("SUCCESS: Connected to Bevy Remote!")
            print("Response:", response.json())
            sys.exit(0)
    except requests.exceptions.ConnectionError:
        pass
    except Exception as e:
        print(f"Error: {e}")
    
    time.sleep(2)
    print(".", end="", flush=True)

print("\nTimeout: Bevy Remote did not start in time.")
sys.exit(1)

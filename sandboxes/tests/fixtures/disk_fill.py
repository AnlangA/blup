# Write ~200 MB to disk — should hit the 100 MB limit
try:
    with open("/tmp/big", "wb") as f:
        f.write(b"0" * 200 * 1024 * 1024)
    print("DISK_WRITE_SUCCEEDED")
except OSError as e:
    print(f"DISK_WRITE_FAILED: {e}")

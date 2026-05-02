try:
    import urllib.request

    urllib.request.urlopen("http://example.com", timeout=5)
    print("NETWORK_ACCESSIBLE")
except Exception:
    print("NETWORK_BLOCKED")

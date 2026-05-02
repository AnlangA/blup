# Allocate ~512 MB to trigger OOM with a 256 MB limit
x = bytearray(512 * 1024 * 1024)
print(len(x))

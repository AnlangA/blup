FROM alpine:3.19

# Install GCC
RUN apk add --no-cache gcc musl-dev

# Create sandbox user
RUN adduser -D -u 1000 sandbox

# Create writable workspace
RUN mkdir -p /workspace && chown sandbox:sandbox /workspace

# Standard runner: read stdin → compile → run
RUN cat > /usr/local/bin/sandbox-run-c << 'SCRIPT' && chmod +x /usr/local/bin/sandbox-run-c
#!/bin/sh
set -e
SRC=$(mktemp /workspace/main_XXXXXX.c)
cat > "$SRC"
gcc -Wall -O0 "$SRC" -o /workspace/a.out
/workspace/a.out
SCRIPT

# Switch to sandbox user
USER sandbox

# Set working directory
WORKDIR /workspace

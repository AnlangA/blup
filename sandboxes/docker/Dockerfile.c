FROM alpine:3.19

# Install GCC
RUN apk add --no-cache gcc musl-dev

# Create sandbox user
RUN adduser -D -u 1000 sandbox

# Create writable workspace
RUN mkdir -p /workspace && chown sandbox:sandbox /workspace

# Standard runner: read stdin → compile → run
RUN printf '#!/bin/sh\nset -e\nSRC=$(mktemp /workspace/main_XXXXXX.c)\ncat > "$SRC"\ngcc -Wall -O0 "$SRC" -o /workspace/a.out\n/workspace/a.out\n' \
    > /usr/local/bin/sandbox-run-c && chmod +x /usr/local/bin/sandbox-run-c

# Switch to sandbox user
USER sandbox

# Set working directory
WORKDIR /workspace

FROM golang:1.22-alpine

# Create sandbox user
RUN adduser -D -u 1000 sandbox

# Create writable workspace
RUN mkdir -p /workspace && chown sandbox:sandbox /workspace

# Standard runner: read stdin → compile → run
RUN printf '#!/bin/sh\nset -e\nDIR=$(mktemp -d /workspace/main_XXXXXX)\ncat > "$DIR/main.go"\ncd "$DIR" && go build -o a.out main.go && ./a.out\n' \
    > /usr/local/bin/sandbox-run-go && chmod +x /usr/local/bin/sandbox-run-go

# Switch to sandbox user
USER sandbox

# Set working directory
WORKDIR /workspace

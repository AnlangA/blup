FROM golang:1.22-alpine

# Create sandbox user
RUN adduser -D -u 1000 sandbox

# Create writable workspace
RUN mkdir -p /workspace && chown sandbox:sandbox /workspace

# Standard runner: read stdin → compile → run
RUN cat > /usr/local/bin/sandbox-run-go << 'SCRIPT' && chmod +x /usr/local/bin/sandbox-run-go
#!/bin/sh
set -e
DIR=$(mktemp -d /workspace/main_XXXXXX)
cat > "$DIR/main.go"
cd "$DIR" && go build -o a.out main.go && ./a.out
SCRIPT

# Switch to sandbox user
USER sandbox

# Set working directory
WORKDIR /workspace

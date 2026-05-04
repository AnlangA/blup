FROM eclipse-temurin:21-jdk-alpine

# Create sandbox user
RUN adduser -D -u 1000 sandbox

# Create writable workspace and output directory
RUN mkdir -p /workspace/out && chown -R sandbox:sandbox /workspace

# Standard runner: read stdin → compile → run
RUN cat > /usr/local/bin/sandbox-run-java << 'SCRIPT' && chmod +x /usr/local/bin/sandbox-run-java
#!/bin/sh
set -e
SRC=$(mktemp /workspace/Main_XXXXXX.java)
cat > "$SRC"
javac "$SRC" -d /workspace/out
CLASSFILE=$(basename "$SRC" .java)
java -cp /workspace/out "$CLASSFILE"
SCRIPT

# Switch to sandbox user
USER sandbox

# Set working directory
WORKDIR /workspace

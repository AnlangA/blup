FROM eclipse-temurin:21-jdk-alpine

# Create sandbox user
RUN adduser -D -u 1000 sandbox

# Create writable workspace and output directory
RUN mkdir -p /workspace/out && chown -R sandbox:sandbox /workspace

# Standard runner: read stdin → compile → run
RUN printf '#!/bin/sh\nset -e\nSRC=$(mktemp /workspace/Main_XXXXXX.java)\ncat > "$SRC"\njavac "$SRC" -d /workspace/out\nCLASSFILE=$(basename "$SRC" .java)\njava -cp /workspace/out "$CLASSFILE"\n' \
    > /usr/local/bin/sandbox-run-java && chmod +x /usr/local/bin/sandbox-run-java

# Switch to sandbox user
USER sandbox

# Set working directory
WORKDIR /workspace

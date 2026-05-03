FROM alpine:3.19

# Install bash
RUN apk add --no-cache bash

# Create sandbox user
RUN adduser -D -u 1000 sandbox

# Switch to sandbox user
USER sandbox

# Set working directory
WORKDIR /workspace

# Default entrypoint
ENTRYPOINT ["bash", "-c"]

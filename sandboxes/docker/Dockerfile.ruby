FROM ruby:3.3-alpine

# Create sandbox user
RUN adduser -D -u 1000 sandbox

# Switch to sandbox user
USER sandbox

# Set working directory
WORKDIR /workspace

# Default entrypoint
ENTRYPOINT ["ruby", "-e"]

#!/bin/sh
# Structured log output helpers

log_info() {
    echo "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"level\":\"INFO\",\"message\":\"$*\"}"
}

log_error() {
    echo "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"level\":\"ERROR\",\"message\":\"$*\"}" >&2
}

#!/bin/sh
# Terminal color helpers

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

pass() { echo "${GREEN}  PASS:${NC} $*"; }
fail() { echo "${RED}  FAIL:${NC} $*"; }
warn() { echo "${YELLOW}  WARN:${NC} $*"; }
info() { echo "${BLUE}  INFO:${NC} $*"; }

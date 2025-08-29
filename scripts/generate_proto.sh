#!/usr/bin/env bash
set -euxo pipefail
# scripts/generate_proto.sh
#
# Idempotent protoc generation script using Buf.
# - Leverages buf.gen.yaml for configuration.
# - Generates Go, gRPC-Gateway, OpenAPI, and Rust outputs.
# - Designed for local dev and CI usage.
#
# Usage:
#   Run from the project root: ./scripts/generate_proto.sh
#
# This script assumes `buf` is available in the environment or run via Docker Compose.

main() {
  echo "Generating protobuf code using buf..."
  buf generate protos
  echo "Protobuf code generation complete."
}

main "$@"
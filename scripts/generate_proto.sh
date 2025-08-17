#!/usr/bin/env bash
# scripts/generate_proto.sh
# Skeleton script to generate protobuf code for Go and TypeScript (grpc-gateway/OpenAPI)
# Make executable: chmod +x scripts/generate_proto.sh
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PROTO_DIR="${ROOT_DIR}/protos"
OUT_DIR_GO="${ROOT_DIR}/gen/go"
OUT_DIR_TS="${ROOT_DIR}/gen/ts"
OUT_DIR_SWAGGER="${ROOT_DIR}/gen/swagger"

mkdir -p "${OUT_DIR_GO}" "${OUT_DIR_TS}" "${OUT_DIR_SWAGGER}"

echo "Generating protobuf code..."
# Example for Go
protoc \
  --proto_path="${PROTO_DIR}" \
  --go_out="${OUT_DIR_GO}" --go_opt=paths=source_relative \
  --go-grpc_out="${OUT_DIR_GO}" --go-grpc_opt=paths=source_relative \
  $(find "${PROTO_DIR}" -name '*.proto')

# Example: generate grpc-gateway / OpenAPI (if plugins installed)
# protoc \
#   --proto_path="${PROTO_DIR}" \
#   --grpc-gateway_out="${OUT_DIR_GO}" --grpc-gateway_opt=paths=source_relative \
#   --openapiv2_out="${OUT_DIR_SWAGGER}" \
#   $(find "${PROTO_DIR}" -name '*.proto')

echo "Protobuf generation completed."
echo "Go artifacts in: ${OUT_DIR_GO}"
echo "TypeScript artifacts in: ${OUT_DIR_TS}"
echo "Swagger artifacts in: ${OUT_DIR_SWAGGER}"
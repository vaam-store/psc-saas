#!/usr/bin/env bash
set -euxo pipefail
# scripts/generate_proto.sh
#
# Idempotent protoc generation script.
# - Verifies required tools are available (protoc and plugins)
# - Generates Go, gRPC-Gateway, and OpenAPI outputs into gen/
# - Designed for local dev and CI usage
#
# Usage:
#   PROTO_ROOT=proto OUT_DIR=gen ./scripts/generate_proto.sh
#
PROTO_ROOT="${PROTO_ROOT:-proto}"
OUT_DIR="${OUT_DIR:-gen}"
GO_OUT_DIR="${OUT_DIR}/go"
TS_OUT_DIR="${OUT_DIR}/ts"
SWAGGER_OUT_DIR="${OUT_DIR}/swagger"
PLUGINS=(protoc-gen-go protoc-gen-go-grpc protoc-gen-grpc-gateway protoc-gen-openapiv2)

fail() {
  echo "ERROR: $*" >&2
  exit 1
}

check_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    return 1
  fi
  return 0
}

check_plugins() {
  local miss=()
  for p in "${PLUGINS[@]}"; do
    if ! check_tool "$p"; then
      miss+=("$p")
    fi
  done
  if [ "${#miss[@]}" -ne 0 ]; then
    echo "Missing protoc plugins: ${miss[*]}"
    echo "You can install Go-based plugins using:"
    echo "  go install google.golang.org/protobuf/cmd/protoc-gen-go@latest"
    echo "  go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest"
    echo "  go install github.com/grpc-ecosystem/grpc-gateway/v2/protoc-gen-grpc-gateway@latest"
    echo "  go install github.com/grpc-ecosystem/grpc-gateway/v2/protoc-gen-openapiv2@latest"
    return 1
  fi
  return 0
}

main() {
  echo "PROTO_ROOT=${PROTO_ROOT}"
  if [ ! -d "${PROTO_ROOT}" ]; then
    fail "Proto root '${PROTO_ROOT}' does not exist"
  fi

  if ! check_tool protoc; then
    fail "protoc is not installed or not on PATH. Install from https://github.com/protocolbuffers/protobuf/releases"
  fi

  if ! check_plugins; then
    fail "One or more required protoc plugins are missing. See instructions above."
  fi

  mkdir -p "${GO_OUT_DIR}" "${TS_OUT_DIR}" "${SWAGGER_OUT_DIR}"

  # Find all .proto files under PROTO_ROOT (exclude gen/ and well-known types)
  mapfile -t protos < <(find "${PROTO_ROOT}" -name '*.proto' -not -path "${OUT_DIR}/*" | sort)
  if [ "${#protos[@]}" -eq 0 ]; then
    echo "No .proto files found under ${PROTO_ROOT}"
    exit 0
  fi

  echo "Found ${#protos[@]} proto files. Generating..."

  # Generate Go code and gRPC service code
  for p in "${protos[@]}"; do
    echo "Generating Go for $p"
    protoc \
      --proto_path="${PROTO_ROOT}" \
      --go_out=paths=source_relative:"${GO_OUT_DIR}" \
      --go-grpc_out=paths=source_relative:"${GO_OUT_DIR}" \
      "$p"
  done

  # Generate gRPC-Gateway (reverse-proxy) Go code (if annotations used)
  # This may generate .pb.gw.go files adjacent to Go outputs
  for p in "${protos[@]}"; do
    echo "Generating gRPC-Gateway for $p"
    protoc \
      --proto_path="${PROTO_ROOT}" \
      --grpc-gateway_out=logtostderr=true,paths=source_relative:"${GO_OUT_DIR}" \
      "$p"
  done

  # Generate OpenAPI (swagger) files (requires protoc-gen-openapiv2)
  # Output will be placed in ${SWAGGER_OUT_DIR} with filenames based on package and service
  for p in "${protos[@]}"; do
    echo "Generating OpenAPI for $p"
    # protoc-gen-openapiv2 supports --openapiv2_out= and a M flag for proto import mapping if needed
    protoc \
      --proto_path="${PROTO_ROOT}" \
      --openapiv2_out="${SWAGGER_OUT_DIR}" \
      "$p"
  done

  # Example placeholder for TypeScript generation using ts-proto or other plugin.
  # If you want TypeScript generation using ts-proto:
  #  - Install plugin (e.g., protoc-gen-ts or use buf+plugins)
  #  - Add invocation here to generate into ${TS_OUT_DIR}
  #
  # Note: Many TypeScript workflows use `buf` or `buf generate` with a buf.gen.yaml file.
  echo "Generation complete. Outputs:"
  echo "  Go: ${GO_OUT_DIR}"
  echo "  Swagger/OpenAPI: ${SWAGGER_OUT_DIR}"
  echo "  TypeScript: ${TS_OUT_DIR} (manual configuration required)"
}

main "$@"
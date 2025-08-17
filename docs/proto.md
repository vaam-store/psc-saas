# Protobuf Tooling & Code Generation

This document describes how to generate language bindings and OpenAPI specs from the project's Protobuf definitions.

Prerequisites
- protoc (Protocol Buffer compiler) installed. Download from https://github.com/protocolbuffers/protobuf/releases
- Go toolchain installed (for Go plugins)
- Recommended: add the following tools to your PATH via `go install`:
  - google.golang.org/protobuf/cmd/protoc-gen-go
  - google.golang.org/grpc/cmd/protoc-gen-go-grpc
  - github.com/grpc-ecosystem/grpc-gateway/v2/protoc-gen-grpc-gateway
  - github.com/grpc-ecosystem/grpc-gateway/v2/protoc-gen-openapiv2

Quickstart
1. From the project root:
   make proto-gen

2. Outputs
- gen/go/: Go `.pb.go` and `.pb.gw.go` files
- gen/swagger/: OpenAPI/Swagger JSON files
- gen/ts/: (manual configuration if using ts-proto or similar)

CI Integration
- The repository includes a GitHub Actions workflow `.github/workflows/ci.yml` that installs protoc and Go-based plugins and runs `make proto-gen` during CI.

Commit strategy
- Generated code can either be committed to the repository or produced as part of CI:
  - Commit generated code: simpler for local development but increases merge conflict surface.
  - Generate in CI: cleaner history and smaller repo, but requires tooling installed locally during development.
- This project defaults to generating in CI and ignores `gen/` via `.gitignore`. If you prefer committing generated outputs, update `.gitignore` accordingly.

Troubleshooting
- If you see "protoc: command not found", ensure protoc is installed and on PATH.
- If plugin binaries are missing, run:
  go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
  go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest
  go install github.com/grpc-ecosystem/grpc-gateway/v2/protoc-gen-grpc-gateway@latest
  go install github.com/grpc-ecosystem/grpc-gateway/v2/protoc-gen-openapiv2@latest

Advanced
- Consider using `buf` for more reproducible generation and linting (`buf build`, `buf generate`, `buf lint`).
- Use a `buf.gen.yaml` file to declaratively manage code generation and plugins.
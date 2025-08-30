# Makefile - project automation helpers
SHELL := /bin/bash
.PHONY: proto-gen proto-clean proto-validate

PROTO_ROOT ?= protos
# OUT_DIR is not used when generating via buf; kept for compatibility
OUT_DIR ?= gen
SCRIPT := ./scripts/generate_proto.sh

proto-gen:
	@echo "Running proto generation with buf..."
	@PROTO_ROOT=$(PROTO_ROOT) OUT_DIR=$(OUT_DIR) $(SCRIPT)

proto-clean:
	@echo "Cleaning generated proto outputs..."
	@rm -rf $(OUT_DIR)/* || true

proto-validate:
	@echo "Validating proto files with buf (if available)..."
	@if command -v buf >/dev/null 2>&1; then \
	  buf lint; \
	else \
	  echo "buf not found; skipping proto validation. Install from https://docs.buf.build/"; \
	fi

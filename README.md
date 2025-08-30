# PSC SaaS - Payments Shared Libraries Monorepo

This monorepo holds shared libraries and service scaffolding for the PSC payments platform.
It includes Protobuf definitions, shared domain types, configuration utilities, observability,
idempotency helpers, provider adapters, retry logic, and CI/CD deployment manifests.

Core directories
- services/      - microservices implementations (per-service repositories planned)
- shared/        - reusable libraries (domain, config, errors, retry, idempotency, observability)
- protos/        - Protobuf schemas (common and service-specific)
- gen/           - Generated artifacts for some languages (if configured)
- deploy/        - Kubernetes / Knative / manifest templates
- scripts/       - helper scripts (proto generation, local dev helpers)
- test/          - shared test harness and integration test configs

Quickstart (local)
1. Install development tools: protoc, buf, docker, kind (optional), go/node/etc.
2. Generate Protobuf artifacts:
   - scripts/generate_proto.sh
3. Run unit tests:
   - make test
4. Run linters & formatters:
   - make lint
   - make fmt

Protobuf & Codegen
- Place canonical protobuf files under [`protos/`](protos/:1)
- Use the provided generation script [`scripts/generate_proto.sh`](scripts/generate_proto.sh:1) to generate code via Buf. Rust code is compiled directly into crates (for example under `crates/packages`) based on `buf.gen.yaml` configuration.

Development workflow
- Follow Conventional Commits for commit messages
- Use feature branches prefixed with `feature/` or `fix/`
- Open a PR and reference related Taskmaster subtasks (use the Taskmaster task IDs)

Contributing
See [`CONTRIBUTING.md`](CONTRIBUTING.md:1) for branch naming, PR guidelines, and how to add tasks to Taskmaster.

CI
Continuous Integration configuration may be added per-repo; ensure lint, tests, and proto generation run in your CI as needed.

License
This project is released under the MIT License.

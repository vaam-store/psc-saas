# Contributing to PSC SaaS

Thank you for contributing to the PSC SaaS monorepo. This document explains the repository workflow, commit conventions, PR expectations, and development practices we follow. Follow these guidelines to ensure smooth collaboration.

## Branching strategy
- Branch name formats:
  - `feature/<short-description>` — new features
  - `fix/<short-description>` — bug fixes
  - `chore/<short-description>` — maintenance or infra tasks
  - `hotfix/<short-description>` — urgent production fixes
  - `release/<version>` — release branches
- Work on one Taskmaster task/subtask per branch when practical and reference the Taskmaster ID in the PR description.
- Keep branches small and focused. Prefer multiple small PRs over a single large one.

## Commit message guidelines
- Follow Conventional Commits: `<type>(scope?): subject`
  - Example: `feat(shared): add Money type using XAF minor units`
  - Allowed types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`, `ci`
- Keep the subject short (<= 72 characters). Use the body to explain motivation, migration notes, or important decisions.
- When appropriate, reference Taskmaster IDs and issue numbers in the commit body.

## Pull Request (PR) process
- Target `master` (or the designated release branch) when opening PRs.
- PR title should be concise and start using the commit type when possible (e.g., `feat(shared): ...`).
- PR description should include:
  - Related Taskmaster task/subtask IDs (e.g., `Task 1.1`)
  - What changed and why
  - How to test locally and any required setup
  - Backwards-compatibility notes and migration steps if applicable
- Request review from at least one maintainer. Add labels (e.g., `user-story`, `ticket`, `breaking-change`) as necessary.
- Use draft PRs for in-progress work.

## Code review checklist
- [ ] Tests added or updated for new behavior
- [ ] Linters & formatters pass
- [ ] No secrets or sensitive data in the diff
- [ ] Public APIs (including Protobuf) documented and backward-compatible where feasible
- [ ] Documentation and README updates included when relevant
- [ ] CI passes for the PR

## Protobuf conventions
- Canonical `.proto` files live in [`protos/`](protos/:1) (note: repo uses `protos/`).
- Maintain backward compatibility for published messages and services where possible.
- When editing Protobufs: update the schema, run codegen, and verify generated artifacts are updated (or generated in CI).
- Use the generator script: [`scripts/generate_proto.sh`](scripts/generate_proto.sh:1) to produce language bindings and OpenAPI if needed.

## Formatting & linting
- Rust: `cargo fmt`, `cargo clippy`
- Go: `gofmt`, `go vet`, `golangci-lint`
- TypeScript/JS: `prettier`, `eslint`
- Run linters locally and fix issues before opening a PR.

## Testing
- Unit tests: run with `cargo test`, `go test ./...`, or the appropriate command for the language.
- Integration tests: use the `test/` harness and local infrastructure (docker-compose, kind, etc.).
- Protobuf schema validation: use `buf lint` or equivalent tools in CI.

## Secrets & credentials
- Never commit secrets, API keys, or `.env` files. Use Vault/KMS in runtime and CI.
- Add common secret file patterns to [`.gitignore`](.gitignore:1).

## CI / CD expectations
- CI must run: lint, format, build, unit tests, and protobuf validation at a minimum.
- For services, CI should also produce container artifacts and optionally publish to a registry on release branches.
- PRs must pass CI before merging.

## Local development tips
- Use `scripts/` for recurring tasks (codegen, local dev helpers).
- Prefer generated artifacts to be produced by CI. If you regenerate artifacts locally, document the steps in the related PR.
- Document per-service local setup in that service's README.

## Repository governance
- Use `CODEOWNERS` to indicate maintainers for subdirectories and services.
- Keep the `.roo/` rules and project conventions up to date if changes to standards are introduced.

## Thank you
We appreciate your contributions. Following these guidelines helps keep the project consistent and maintainable.

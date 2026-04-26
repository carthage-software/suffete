# Contributing to Suffete

Thanks for your interest in contributing. Suffete is a young, fast-moving project, and help is very welcome. Please read this document first so we can keep the work productive.

## Code of Conduct

This project adheres to the [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold it.

## Issues

Before filing an issue, please check whether it is already reported. When opening a new one, include enough information to reproduce the behavior. Ideally, a small Rust snippet that calls into `suffete` and exhibits the problem, plus what you expected to happen.

For bugs, use the [bug report](.github/ISSUE_TEMPLATE/bug_report.yml) template. For feature ideas, use the [feature request](.github/ISSUE_TEMPLATE/feature_request.yml) template.

## Getting Started

1. **Discuss first.** For anything beyond a small fix, open an issue before starting work. The type system is in active design and the answer to "is this the right shape?" often is not obvious.
2. **Fork and clone** the repository.
3. **Set up your environment.** You need a recent Rust toolchain (see `rust-version` in `Cargo.toml`) and [Just](https://github.com/casey/just). A Nix `flake.nix` is provided. Run `nix develop`, or use [direnv](https://direnv.net/), to drop into a reproducible shell.
4. **Create a branch** off `main`.
5. **Make your change.**
6. **Verify.** Run `just test` and `just check` locally before pushing.
7. **Commit** with a clear, present-tense message describing *why* the change is being made.
8. **Push and open a pull request.**

## Pull Requests

- **Tests are required** for bug fixes and new functionality. Run `just test` and make sure the existing suite still passes.
- **Formatting and lints** are enforced by CI. Run `just check` (or `just fix` to auto-apply what can be auto-applied) before pushing.
- **Authorship matters.** Every commit on a pull request must be authored by a real human we can reach. AI assistance is welcome, but the commit author and any `Co-Authored-By:` trailers must be people, not models. CI will reject the PR otherwise.
- **License.** By submitting a pull request, you agree that your contribution is dual-licensed under MIT and Apache-2.0, matching the rest of the repository.

## Security

Do not open public issues for security vulnerabilities. See [SECURITY.md](SECURITY.md) for the disclosure process.

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this project is

vicraft is a Rust CLI that orchestrates a spec-driven AI development workflow on top of [Aider](https://aider.chat). It does not do AI work itself — it shells out to the `aider` binary and structures the artifacts of each step (issue → spec → plan → impl → review → commit → PR). The user-facing workflow is documented in `README.md`; this file focuses on the internal architecture.

## Build / run

- `cargo build` (or `cargo build --release`)
- `cargo check` — fast type-checking
- `cargo run -- <subcommand>` — run a CLI command without installing
- `cargo clippy`, `cargo fmt`

There is currently **no test suite** (no `tests/` dir, no `#[test]` modules). Don't hunt for one; `cargo check` + `cargo clippy` are the main correctness gates.

## Architecture

- **Entry point** — `src/main.rs`: tokio runtime, parses CLI via `clap` (`src/cli.rs`), loads global config (`src/config.rs`), dispatches to a `commands::<name>::run(...)` function.
- **Command modules** — `src/commands/{init,scan,new_issue,spec,plan,implement,review,commit,pr,clear_context,skills}.rs`: one file per subcommand, each exposing a `run(...)` (usually `async`) that owns the full lifecycle of that step. `src/commands/mod.rs` just re-exports them.
- **Aider wrapper** — `src/aider.rs`: the `AiderCommand` builder (`ask` vs `edit` mode) is the *only* place that spawns the `aider` binary. Every AI-invoking command goes through this — don't shell out to `aider` elsewhere. It always passes `--no-auto-commits`, `--yes-always`, `--no-pretty`. `default_read_files()` and `relevant_skills()` decide which context files get attached to each Aider call.
- **Git ops** — `src/git.rs`: wraps `git2` for branch/commit/diff operations used by `impl`, `commit`, and `pr`.
- **Templates** — `src/templates.rs`: string templates for the markdown files vicraft *generates into the target project* (`.issues/`, `.specs/`, `.plans/`, `.reviews/`, `.implementations/`).
- **Config** — `src/config.rs`: global TOML at `~/.config/vicraft/config.toml`. Every struct uses `serde(default)` so missing fields auto-fill; `Config::default()` is the source of truth for defaults (model, branch prefix, etc.).

## Key invariants / non-obvious things

- **WIP commit strategy.** `vicraft impl` always runs Aider with `--no-auto-commits` and creates a single `wip: <task-id>` commit. `vicraft commit` then *amends* that WIP commit rather than creating a new one — this preserves "one clean commit per task". Don't break this when touching `commands/implement.rs` or `commands/commit.rs`.
- **Two operating directories.** This Rust repo is where vicraft is *developed*, but at runtime vicraft operates on a *target project* (the user's cwd), reading and writing `.aider/`, `.issues/`, `.specs/`, `.plans/`, `.reviews/`, `.implementations/`. Don't confuse paths that belong to this repo with paths vicraft creates in a user's project.
- **Artifact dirs are committed** in the target repo, *except* `.aider/context/` which is auto-generated and gitignored. `commands/init.rs` and `templates.rs` should respect that.
- **Linear integration is optional.** `LinearConfig` has empty defaults; `vicraft spec LINEAR-123` only works if `linear.api_token` (or the `SPEQ_LINEAR_TOKEN` env var) is set.
- **Skill matching is deliberately naive.** `aider::relevant_skills()` is a plain keyword lookup, not semantic search — keep it dependency-free if you extend it.

## End-to-end testing

Running the actual workflow (`vicraft spec`, `impl`, `review`, …) requires:

- `aider` in `$PATH` (`pip install aider-chat`)
- An Ollama instance serving `qwen3-coder:30b`, or a different model configured in `~/.config/vicraft/config.toml`
- `git`, and `gh` for `vicraft pr`

Without these, only `cargo build` / `cargo check` / `cargo clippy` are meaningful — don't try to execute `vicraft impl` in a bare environment.

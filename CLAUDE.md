# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test

```bash
cargo build                        # build workspace
cargo test                         # run all 15 unit tests
cargo test -p ab-self-upgrade      # library tests only
cargo run -p ab-self-upgrade-example  # run example binary
```

## Project Overview

A/B Self Upgrade Library — a Rust library that enables applications to automatically download, verify, and install new versions via symlink swap while keeping the old binary running until the new one confirms healthy.

## Workspace Structure

- `crates/ab-self-upgrade/` — library crate (the public API)
- `crates/ab-self-upgrade-example/` — example binary demonstrating usage

## Key Design Constraints

- **Zero-downtime**: The old program MUST continue running during the entire upgrade process. The new version only takes over after it starts successfully and reports heartbeat OK.
- **A/B slots**: Two install directories (`slot_a`, `slot_b`) with a `current` symlink. Upgrade writes to inactive slot, then atomically swaps the symlink via `rename()`.
- **Versioning**: Semantic versioning (`major.minor.patch`), e.g. `0.1.2`, `1.0.0`
- **Download verification**: SHA-256 checksum verification is mandatory before installing
- **State reporting**: Upgrade success/failure must be reported to the cloud; on failure the old version keeps running
- **State persistence**: Upgrade state machine persisted to `state.json` — survives restarts and resumes interrupted upgrades

## Cloud API Contract

- Metadata endpoint: `GET ${api_domain}${auto_upgrade_metadata}` — returns JSON with `latest_version`, per-platform `download_url`, and `checksum` (format: `sha256:<hex>`)
- State reporting: POST to `state_report_url` with JSON body `{status, from_version, to_version, error}`

## Public API

- `UpgradeConfig` — parse config via `from_str()` / `from_file()`, chain builder methods (`with_install_dir`, `with_binary_name`, `with_current_version`, `with_state_report_url`)
- `UpgradeEngine::new(config)` — create engine, initializes A/B directory layout
- `UpgradeEngine::check_and_upgrade()` — single upgrade cycle: fetch → compare → download → verify → install → swap
- `UpgradeEngine::confirm_healthy()` — call from new binary to finalize switch
- `UpgradeEngine::run_loop(shutdown)` — periodic upgrade loop on configurable interval

## PRD Reference

Full requirements are in `ab-self-upgrade.md` (written in Chinese).

# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Unreleased]

### Documentation
- Standardize README to 9-section template and establish AST-grounded 9-document architecture

## [v0.1.0-beta.4] - YYYY-MM-DD

### Added
- Initialized local backlog store with `SH` prefix for scoped task tracking (`sawhorse-backlog`).

### Fixed
- No fixes in this release.

### Changed
- Modularized `OpenAiCompatProvider` with structured request logging and reasoning diagnostic hints (`sawhorse-providers`) [GB-80].
- Migrated scoped tasks and design plans to the updated backlog model (`sawhorse-core`).

### Maintenance
- Updated release history with alpha and beta milestones (`sawhorse`).

## [v0.1.0-beta.3] - 2026-10-26

### Fixed
- `sawhorse-core`: support data-wrapped choices in OpenAI compatible provider (GB-45)
- `sawhorse-core`: resolve secrets from .gaugeblock/.env in OpenAiCompatProvider (GB-41)

### Changed
- `sawhorse-core`: streamline openai provider to adhere to 250 LOC rule (GB-45)

## [v0.1.0-beta.2] - 2026-10-24

### Added
- `sawhorse-core`: support unified `.gaugeblock/config.toml` configuration discovery (GB-36).
- `sawhorse-cli`: enable `--version` attribute on Clap CLI entrypoint.

## [v0.1.0-beta.1] - 2026-10-22

### Added
- `sawhorse-core`: provider-agnostic 3-mode LLM runtime (`Mode 1: Candle`, `Mode 2: Local GGUF`, `Mode 3: OpenAI-Compatible`) (GB-1).
- `sawhorse-core`: role-based model routing (`Planner`, `System1Router`, `HoleFiller`, `Oracle`).
- `sawhorse-cli`: diagnostic subcommands `sawhorse ping`, `sawhorse bench`, `sawhorse complete`, `sawhorse init`.

## [0.0.1-alpha.1] - 2026-10-21

### Added
- Initial v0.0.1-alpha.1 namespace reservation on crates.io and GitHub repository scaffold.



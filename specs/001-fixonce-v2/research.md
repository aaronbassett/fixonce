# Research: FixOnce v2

## R1: Rust CLI Framework

**Decision**: `clap` v4 for CLI argument parsing
**Rationale**: Industry standard, derive macros for ergonomic definition, built-in help generation, shell completion support. Alternatives (structopt, argh) are either deprecated or less maintained.
**Alternatives**: structopt (deprecated, merged into clap), argh (Google, minimal features), pico-args (too minimal)

## R2: TUI Framework

**Decision**: `ratatui` + `crossterm` for terminal UI
**Rationale**: ratatui is the actively maintained fork of tui-rs, with the largest Rust TUI community. crossterm provides cross-platform terminal manipulation. Together they handle rendering, input, and terminal lifecycle.
**Alternatives**: cursive (higher-level but less flexible), dioxus-tui (experimental), tuirealm (less community)

## R3: HTTP Client

**Decision**: `reqwest` with tokio async runtime
**Rationale**: Most popular Rust HTTP client. Native async support. rustls for TLS (no OpenSSL dependency, enabling static linking). Connection pooling built-in.
**Alternatives**: ureq (synchronous, simpler), hyper (lower-level), surf (less maintained)

## R4: Cryptography for Challenge-Response Auth

**Decision**: `ed25519-dalek` for Ed25519 keypair generation and signing
**Rationale**: Pure Rust implementation, widely audited, no C dependencies (important for static linking). Ed25519 is fast, compact keys, and the standard for SSH-style auth.
**Alternatives**: ring (also good, but has C code complicating static builds), p256 (NIST curves, less common for this use case)

## R5: TOON Output Format

**Decision**: `toon` crate (https://crates.io/crates/toon)
**Rationale**: Purpose-built for token-efficient LLM output. User explicitly requested TOON support. Crate provides serialization matching the TOON spec.
**Alternatives**: Custom implementation (unnecessary when crate exists), MessagePack (not LLM-optimized)

## R6: Supabase Edge Function Patterns

**Decision**: Deno-native edge functions with shared utilities in `_shared/`
**Rationale**: Supabase edge functions run on Deno Deploy. Shared auth/validation/error utilities prevent duplication across ~15 functions. Zod for schema validation is the standard in the Supabase ecosystem.
**Alternatives**: Single monolithic edge function with routing (worse isolation, harder to deploy independently)

## R7: Embedding Model

**Decision**: VoyageAI voyage-code-3 (1024 dimensions)
**Rationale**: Purpose-built for code understanding. 1024 dimensions provide good accuracy/performance balance. User explicitly chose this in ideation. Generated client-side in the CLI, not in edge functions.
**Alternatives**: OpenAI text-embedding-3-large (1536 dims, not code-specialized), Cohere embed-v3 (general purpose)

## R8: Hybrid Search Algorithm

**Decision**: Reciprocal Rank Fusion (RRF) in Postgres RPC function
**Rationale**: RRF is simple, effective, and doesn't require score normalization between FTS and vector search. Implemented as a single Postgres function for single-trip performance. The formula: `1/(k + rank_fts) + 1/(k + rank_vector)` where k=60 is standard.
**Alternatives**: Weighted linear combination (requires score normalization), learned ranking (over-engineered for v1)

## R9: Secret Encryption

**Decision**: AES-256-GCM via Web Crypto API in edge functions
**Rationale**: Threat model requires secrets to be unrecoverable from a database dump. Edge function encryption with master key as Supabase env secret provides true layer separation. Web Crypto API is built into Deno, zero dependencies.
**Alternatives**: pgsodium (keys also in DB — doesn't survive RLS misconfiguration), client-side encryption (CLI would need the master key)

## R10: Local Private Key Storage

**Decision**: OS keyring via `keyring` crate (macOS Keychain, Linux Secret Service, Windows Credential Manager), with fallback to encrypted file in `~/.config/fixonce/`
**Rationale**: The CLI's own Ed25519 private key (NOT server secrets) needs local storage. OS keyring is the most secure option. File fallback for headless environments (CI servers).
**Alternatives**: Plain file (insecure), environment variable (ephemeral, lost on restart), no local storage (re-auth every time)

## R11: Session Transcript Format

**Decision**: Parse Claude Code's native JSON session log format
**Rationale**: Claude Code stores session data in a structured format. The CLI reads this directly rather than requiring a custom export step. Format details may evolve — parser should be resilient to unknown fields.
**Alternatives**: Custom export format (extra step for users), raw text parsing (fragile)

## R12: Decay Function

**Decision**: Exponential decay with configurable half-life, default 30 days
**Rationale**: `decay_score = initial_score * e^(-λt)` where λ = ln(2)/half_life and t = days since last access. Simple, well-understood, single parameter to tune. Event-driven acceleration multiplies λ by a boost factor.
**Alternatives**: Linear decay (too aggressive early, too slow late), step function (discontinuities), no decay (noise accumulation)

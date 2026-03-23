.PHONY: check build test fmt lint clippy

## Run all quality checks (Rust: fmt, clippy, test; Deno: fmt, lint)
check: fmt-check lint test deno-check

## Build all Rust workspace crates
build:
	cargo build

## Run Rust tests
test:
	cargo test

## Format Rust source code (and Deno edge functions if they exist)
fmt:
	cargo fmt
	@if find supabase/functions -name '*.ts' 2>/dev/null | grep -q .; then \
		deno fmt supabase/functions; \
	fi

## Check formatting without making changes (Rust + Deno)
fmt-check:
	cargo fmt --check
	@if find supabase/functions -name '*.ts' 2>/dev/null | grep -q .; then \
		deno fmt --check supabase/functions; \
	fi

## Run Clippy lints (deny warnings)
lint:
	cargo clippy -- -D warnings

## Alias for lint
clippy: lint

## Run Deno fmt check and lint (only if .ts files exist)
deno-check:
	@if find supabase/functions -name '*.ts' 2>/dev/null | grep -q .; then \
		deno fmt --check supabase/functions && deno lint supabase/functions; \
	else \
		echo "No Deno edge function files found, skipping Deno checks."; \
	fi

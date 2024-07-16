.PHONY: check format test lint fix

check:
	@cargo check

format:
	@cargo fmt

test:
	@cargo test

lint:
	@cargo clippy

checks: check format test lint
	@git status
	@echo looks good enough to raise a PR 👍
	@echo awesome work! 😍

fix:
	@cargo clippy --fix --allow-dirty

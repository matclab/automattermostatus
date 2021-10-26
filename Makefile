
all: fmt doc clippy

fmt:
	cargo fmt

clippy:
	cargo clippy 

doc: README.md
	 cargo doc  --no-deps --bins

.PHONY: target/debug/automattermostatus

target/debug/automattermostatus:
	cargo build

README.md: target/debug/automattermostatus
	mdsh  --work_dir .


all: fmt doc clippy

fmt:
	cargo fmt

clippy:
	cargo clippy 

.PHONY: doc
doc: README.md doc/automattermostatus.1
	 cargo doc  --no-deps --bins

doc/automattermostatus.1: target/debug/automattermostatus doc/override.h2m
	help2man --include doc/override.h2m --output=$@ ./target/debug/automattermostatus


.PHONY: target/debug/automattermostatus

target/debug/automattermostatus:
	cargo build

README.md: target/debug/automattermostatus
	mdsh  --work_dir .

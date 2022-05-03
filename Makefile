SHELL=bash

.DEFAULT_GOAL := all
.PHONY: help

help: ### Print this help message
	@grep -h -E '^[^ :]+:.*?## .*$$' $(MAKEFILE_LIST) \
	| sed -n 's/^\(.*\):\(.*\)##\(.*\)/\1▅\3/p' \
	| column -t  -s '▅'

all: fmt doc clippy ### format + doc + clippy

fmt: ### format code
	cargo fmt

clippy: ### run clippy
	cargo clippy 

.PHONY: doc
doc: README.md doc/automattermostatus.1 ### make man page and update README
	 cargo doc  --no-deps --bins --lib

doc/automattermostatus.1: target/debug/automattermostatus doc/override.h2m
	help2man --include doc/override.h2m --output=$@ ./target/debug/automattermostatus


.PHONY: target/debug/automattermostatus

target/debug/automattermostatus:
	cargo build

README.md: target/debug/automattermostatus
	mdsh  --work_dir .

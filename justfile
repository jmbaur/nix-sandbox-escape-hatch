export RUST_LOG := "debug"
export RUNTIME_DIRECTORY := `mktemp -d`
export SYSTEMD_SOCKET_ACTIVATE_ADDR := "[::1]:8080"

help:
	just --list

build:
	cargo build

server: build
	systemd-socket-activate \
	--accept \
	--listen="{{SYSTEMD_SOCKET_ACTIVATE_ADDR}}" \
	--setenv=RUNTIME_DIRECTORY \
	--setenv=RUST_LOG \
	{{justfile_directory()}}/target/debug/nix-sandbox-escape-hatch server \
	{{justfile_directory()}}/test/builder.bash

client: build
	{{justfile_directory()}}/target/debug/nix-sandbox-escape-hatch client \
	foobar

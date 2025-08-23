FROM rust:slim-bookworm

RUN apt-get update && apt-get install -y libasound2-dev libpulse0 pkg-config \
    libdbus-1-dev curl \
&&  rustup component add clippy rustfmt \
&&  curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
&&  cargo binstall cargo-nextest  --secure \
&&  rm /usr/local/cargo/bin/cargo-binstall \
&&  rm -rf /var/lib/apt/lists/*

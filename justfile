default:
    just --list

watch:
    watchexec -w src -r --clear=reset cargo run

# install the cli locally
install:
    cargo install --path .

run:
    cargo run --bin mmr

doc:
    cargo doc --no-deps --open

test:
    cargo test --all-features

check:
    pre-commit run --all-files

fix:
    cargo fmt --all
    cargo clippy --fix --all-features --allow-dirty --allow-staged
    cargo fix --allow-dirty --allow-staged

default:
    just --list

watch:
    watchexec -w src -r --clear=reset cargo run
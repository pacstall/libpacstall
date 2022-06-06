#!/usr/bin/env just --justfile

# Setup development environment
setup:
    @echo "Installing nightly \`rustfmt\`"
    @rustup toolchain install nightly --component rustfmt
    @echo "Nightly \`rustfmt\` successfully installed!"

    @echo "Installing \`pre-commit\`"
    @pip install pre-commit
    @pre-commit install
    @echo "\`pre-commit\` hooks successfully installed!"

    @echo "Installing \`codespell\`"
    @pip install codespell
    @echo "\`codespell\` successfully installed!"

    @echo "Development environment installed successfully!"

# Build the project
build +ARGS="":
    @cargo build {{ARGS}}
    @echo "Successfully built the project!"

# Run checks
check: (spellcheck "") (fmt "--check") clippy test
    @echo "Checks were successful!"

# Test the project
test +ARGS="":
    @cargo test --all-features --workspace {{ARGS}}

# Lint the codebase
clippy +ARGS="":
    @cargo clippy --all-targets --all-features --workspace -- --deny warnings --deny clippy::pedantic {{ARGS}}
    @echo Lint successful!

# Format the codebase
fmt +ARGS="": spellcheck
    @cargo +nightly fmt --all -- {{ARGS}}
    @echo Codebase formatted successfully!

# Spellcheck the codebase
spellcheck +ARGS="--write-changes":
    @codespell  --builtin clear,rare,informal,code -I .codespellignore --skip target* {{ARGS}}
    @echo Spellings look good!

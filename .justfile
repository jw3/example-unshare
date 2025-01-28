release:
    cargo build --bins --release --target x86_64-unknown-linux-musl

doc:
    pandoc readme.md .local/* -o namespaces.pdf

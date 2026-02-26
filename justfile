name := 'cosmic-matrix'
appid := 'com.cosmic.CosmicMatrix'

# Build debug
build:
    cargo build

# Build release
release:
    cargo build --release

# Run debug
run:
    cargo run

# Install
install:
    install -Dm0755 target/release/{{name}} /usr/local/bin/{{name}}
    install -Dm0644 data/{{appid}}.desktop /usr/share/applications/{{appid}}.desktop
    install -Dm0644 data/{{appid}}.metainfo.xml /usr/share/metainfo/{{appid}}.metainfo.xml

# Clean
clean:
    cargo clean

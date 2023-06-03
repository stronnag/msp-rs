
build:
	cargo build --release

# Static build for Linux, requires rust-musl / musl / kernel-headers-musl
static:
	cargo build --release --target=x86_64-unknown-linux-musl

install:
	cargo install --path . --root $$HOME/.local/ --force

debug:
	cargo build

windows:
	cargo build --target x86_64-pc-windows-gnu --release

clean:
	cargo clean

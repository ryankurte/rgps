

daemon-linux-x64:
	cross build --manifest-path daemon/Cargo.toml --target x86_64-unknown-linux-gnu --release

deb-linux-x64: daemon-linux-x64
	cargo deb --manifest-path daemon/Cargo.toml --target x86_64-unknown-linux-gnu --release --no-build

daemon-linux-aarch64:
	cross build --manifest-path daemon/Cargo.toml --target aarch64-unknown-linux-gnu --release

deb-linux-aarch64: daemon-linux-aarch64
	cargo deb --manifest-path daemon/Cargo.toml --target aarch64-unknown-linux-gnu --release --no-build

daemon-linux-armv7:
	cross build --manifest-path daemon/Cargo.toml --target armv7-unknown-linux-gnueabihf --release

deb-linux-armv7: daemon-linux-armv7
	cargo deb --manifest-path daemon/Cargo.toml --target armv7-unknown-linux-gnueabihf --release --no-build

debs: deb-linux-x64 deb-linux-aarch64 deb-linux-armv7

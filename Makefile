

daemon-linux-x64:
	cross build --manifest-path daemon/Cargo.toml --target x86_64-unknown-linux-gnu --release

deb-linux-x64: daemon-linux-x64
	cargo deb --manifest-path daemon/Cargo.toml --target x86_64-unknown-linux-gnu --release --no-build

daemon-linux-aarch64:
	cross build --manifest-path daemon/Cargo.toml --target aarch64-unknown-linux-musl --release

deb-linux-aarch64: daemon-linux-aarch64
	cargo deb --manifest-path daemon/Cargo.toml --target aarch64-unknown-linux-musl --release --no-build

daemon-linux-armv7:
	cross build --manifest-path daemon/Cargo.toml --target armv7-unknown-linux-musleabihf --release

deb-linux-armv7: daemon-linux-armv7
	cargo deb --manifest-path daemon/Cargo.toml --target armv7-unknown-linux-musleabihf --release --no-build

debs: deb-linux-x64 deb-linux-aarch64 deb-linux-armv7

build-image:
	docker build -t ghcr.io/ryankurte/rgps/build .

run-image:
	docker run --rm -it -v $(shell pwd):/work --workdir=/work ghcr.io/ryankurte/rgps/build

push-image:
	docker push ghcr.io/ryankurte/rgps/build

sbom:
	cargo-sbom --output-format=cyclone_dx_json_1_4 > rgps.json

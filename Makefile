build:
	cd rust && cross build --target x86_64-pc-windows-gnu

build-release:
	cd rust && cross build --target x86_64-pc-windows-gnu --release

link-debug:
	./link.sh debug

link-release:
	./link.sh debug

install:
	echo "install"

setup:
	rustup default stable
	docker version


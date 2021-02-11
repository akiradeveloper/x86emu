build:
	nasm main.asm

run: build
	cargo run main

dump: build
	objdump -m i386 -b binary -D main
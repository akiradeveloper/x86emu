.PHONY: build
build: 
	nasm main.asm

dump: build
	objdump -m i386 -b binary -D main

run: build
	cargo run main
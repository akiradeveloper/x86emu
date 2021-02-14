.PHONY: build
build: 
	rm -f test.o crt.o
	gcc -march=i386 -m32 -nostdlib -fno-pie -fno-asynchronous-unwind-tables -fno-stack-protector -c test.c
	nasm -f elf crt.asm
	ld -m elf_i386 -s --entry=start --oformat=binary -Ttext 0x7c00 -o main crt.o test.o

dump: build
	objdump -m i386 -b binary -D main

run: build
	cargo run main
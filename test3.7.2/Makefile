.PHONY: build
build: 
	rm -f test.o crt0.o
	gcc -march=i386 -m32 -nostdlib -fno-pie -fno-asynchronous-unwind-tables -fno-stack-protector -c test.c
	nasm -f elf crt0.asm
	ld -m elf_i386 -s --entry=start --oformat=binary -Ttext 0x7c00 -o main crt0.o test.o

dump: build
	objdump -m i386 -b binary -D main

run: build
	cargo run main
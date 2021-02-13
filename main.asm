BITS 32
start:
    org 0x7c00
    sub esp, 16
    mov ebp, esp
    mov eax, 2
    mov dword [ebp+4], 5
    mov dword [ebp+4], eax
    mov esi, [ebp+4]
    inc dword [ebp+4]
    mov edi, [ebp+4]
    jmp 0
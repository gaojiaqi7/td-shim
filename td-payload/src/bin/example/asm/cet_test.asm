.section .text
.global cet_test
cet_test:
    mov rax, rsp
    mov rcx, 0x40
cet_loop:
    add rax, rcx
    mov dword ptr [rax], 0xa
    dec rcx
    cmp rcx, 0
    jnz cet_loop
    ret
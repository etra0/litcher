.data
; Jumpback addr.
EXTERN overwrite_tonemapping_jmb: qword
EXTERN overwrite_tonemapping_val: dword
EXTERN overwrite_tonemapping_enable: byte

.code
overwrite_tonemapping PROC
    pushf
    push rbx
    push rcx

    ; Check that is enabled first
    mov bl, byte ptr [overwrite_tonemapping_enable]
    test bl, bl
    jz original


    ; we have the stuff in RAX
    ; RAX + 1840 -> byte active
    ; RAX + 1840 + 2D8 + 8 -> ptro a scale
    lea rbx, [rax+1b20h]
    mov rcx, [rbx]
    add rcx, 14h
    mov ebx, dword ptr [overwrite_tonemapping_val]
    mov dword ptr [rcx], ebx

    ; Same with RDI
    lea rbx, [rdi+1b20h]
    mov rcx, [rbx]
    add rcx, 14h
    mov ebx, dword ptr [overwrite_tonemapping_val]
    mov dword ptr [rcx], ebx

original:
    pop rcx
    pop rbx
    popf
    movss xmm0, dword ptr [rdi+5108h]
    movss dword ptr [rdi+5120h], xmm0
    jmp [overwrite_tonemapping_jmb]


overwrite_tonemapping ENDP

END

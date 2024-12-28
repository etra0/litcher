.data
; Jumpback addr.
EXTERN overwrite_tonemapping_jmb: qword
EXTERN overwrite_tonemapping_val: dword
EXTERN overwrite_tonemapping_enable: byte

.code
overwrite_tonemapping PROC
    test rdx, rdx
    je if_null

    cmp [rdx + 08h],  r14
    je if_null

    mov rdx, [rdx + 08h]
    add rdx, 68h ; <- now RDX contains what we need.

    ; -- our code
    pushf
    push rbx
    push rcx

    mov bl, byte ptr [overwrite_tonemapping_enable]
    test bl, bl
    jz @f

    ; we have the stuff in RDX
    lea rbx, [rdx+1b20h]
    mov rcx, [rbx]
    add rcx, 14h
    mov ebx, dword ptr [overwrite_tonemapping_val]
    mov dword ptr [rcx], ebx
    ; --

    @@:
    pop rcx
    pop rbx
    popf

    jmp continue

    if_null:
    mov rdx, r14

    continue:
    mov r15, [rsp + 188h]

original:
    jmp [overwrite_tonemapping_jmb]


overwrite_tonemapping ENDP

END


; ORIGINAL CODE - INJECTION POINT: witcher3.exe+166A308
; witcher3.exe+166A2E1: F3 41 0F 5D C3              - minss xmm0,xmm11
; witcher3.exe+166A2E6: F3 0F 59 03                 - mulss xmm0,[rbx]
; witcher3.exe+166A2EA: F3 0F 11 03                 - movss [rbx],xmm0
; witcher3.exe+166A2EE: EB 06                       - jmp witcher3.exe+166A2F6
; witcher3.exe+166A2F0: C7 03 00 00 80 3F           - mov [rbx],3F800000
; witcher3.exe+166A2F6: 48 8B 96 20 02 00 00        - mov rdx,[rsi+00000220]
; witcher3.exe+166A2FD: 48 85 D2                    - test rdx,rdx
; witcher3.exe+166A300: 74 10                       - je witcher3.exe+166A312
; witcher3.exe+166A302: 4C 39 72 08                 - cmp [rdx+08],r14
; witcher3.exe+166A306: 74 0A                       - je witcher3.exe+166A312
; // ---------- INJECTING HERE ----------
; witcher3.exe+166A308: 48 8B 52 08                 - mov rdx,[rdx+08]
; // ---------- DONE INJECTING  ----------
; witcher3.exe+166A30C: 48 83 C2 68                 - add rdx,68
; witcher3.exe+166A310: EB 03                       - jmp witcher3.exe+166A315
; witcher3.exe+166A312: 49 8B D6                    - mov rdx,r14
; witcher3.exe+166A315: 4C 8B BC 24 88 01 00 00     - mov r15,[rsp+00000188]
; witcher3.exe+166A31D: 49 8B CF                    - mov rcx,r15
; witcher3.exe+166A320: E8 BB 62 CF FE              - call witcher3.exe+3605E0
; witcher3.exe+166A325: 44 39 B6 A0 02 00 00        - cmp [rsi+000002A0],r14d
; witcher3.exe+166A32C: 0F 86 96 03 00 00           - jbe witcher3.exe+166A6C8
; witcher3.exe+166A332: 48 89 BC 24 68 01 00 00     - mov [rsp+00000168],rdi
; witcher3.exe+166A33A: 44 0F 29 84 24 00 01 00 00  - movaps [rsp+00000100],xmm8

.data
; Jumpback addr.
EXTERN overwrite_tonemapping_jmb: qword
EXTERN overwrite_tonemapping_val: dword
EXTERN overwrite_tonemapping_enable: byte

.code
overwrite_tonemapping PROC
    test rax, rax
    je if_null

    mov rcx, [rax + 08h]
    lea rdx, [rcx + 70h] ; <- rdx has our value now: CAreaEnvironmentParams.

    ; -- our code
    pushf
    push rbx
    push rcx

    mov bl, byte ptr [overwrite_tonemapping_enable]
    test bl, bl
    jz @f

    ; we have the stuff in RDX
    lea rbx, [rdx+3AB0h]
    mov rcx, [rbx]
    add rcx, 14h
    mov ebx, dword ptr [overwrite_tonemapping_val]
    mov dword ptr [rcx], ebx
    ; --

    @@:
    pop rcx
    pop rbx
    popf

    test rcx, rcx
    jne continue

    if_null:
    mov rdx, r15

    continue:
    mov r13, [rbp + 0A8h]

original:
    jmp [overwrite_tonemapping_jmb]


overwrite_tonemapping ENDP

END


; witcher3.exe+22B2559 - 48 85 C0              - test rax,rax
; witcher3.exe+22B255C - 74 0D                 - je witcher3.exe+22B256B
; witcher3.exe+22B255E - 48 8B 48 08           - mov rcx,[rax+08]
; witcher3.exe+22B2562 - 48 8D 51 70           - lea rdx,[rcx+70]
; witcher3.exe+22B2566 - 48 85 C9              - test rcx,rcx
; witcher3.exe+22B2569 - 75 03                 - jne witcher3.exe+22B256E
; witcher3.exe+22B256B - 49 8B D7              - mov rdx,r15
; witcher3.exe+22B256E - 4C 8B AD A8000000     - mov r13,[rbp+000000A8]
; witcher3.exe+22B2575 - 49 8B CD              - mov rcx,r13
; witcher3.exe+22B2578 - E8 83D95FFE           - call witcher3.exe+8AFF00
; witcher3.exe+22B257D - 45 39 BE D0020000     - cmp [r14+000002D0],r15d
; 

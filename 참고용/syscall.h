#ifndef SYSCALL_H
#define SYSCALL_H

#include <stdint.h>

#define MSR_EFER 0xC0000080
#define MSR_STAR 0xC0000081
#define MSR_LSTAR 0xC0000082
#define MSR_SFMASK 0xC0000084
#define MSR_KERNEL_GS_BASE 0xC0000102

#define EFER_SCE 1 // System Call Extensions

void Syscall_Init();

// Known Sycalls
#define SYSCALL_CLEAR 0
#define SYSCALL_PRINT 1
#define SYSCALL_EXEC 2
#define SYSCALL_TERMINATE 3
#define SYSCALL_HALT 4
#define SYSCALL_NVME_READ 5
#define SYSCALL_NVME_WRITE 6
#define SYSCALL_KMALLOC 7
#define SYSCALL_KFREE 8

#endif

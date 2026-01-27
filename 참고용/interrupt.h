#ifndef INTERRUPT_H
#define INTERRUPT_H

#include <stdint.h>

#define INT_TIMER 0x40

typedef struct {
  uint16_t offset_low;
  uint16_t selector;
  uint8_t ist;
  uint8_t type_attr;
  uint16_t offset_mid;
  uint32_t offset_high;
  uint32_t zero;
} __attribute__((packed)) IDTEntry;

typedef struct {
  uint16_t limit;
  uint64_t base;
} __attribute__((packed)) IDTPointer;

typedef struct {
  uint64_t r15, r14, r13, r12, r11, r10, r9, r8;
  uint64_t rsi, rdi, rbp, rdx, rcx, rbx, rax;
  uint64_t int_no, err_code;
  uint64_t rip, cs, rflags, rsp, ss;
} InterruptFrame;

typedef void (*InterruptHandler)(InterruptFrame **frame);

extern const char *exception_messages[];

void IDT_Init();
void IDT_SetGate(uint8_t vector, void *handler, uint16_t selector,
                 uint8_t type_attr);
void Interrupt_RegisterHandler(uint8_t vector, InterruptHandler handler);

#endif

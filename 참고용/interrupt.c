#include "interrupt.h"
#include "gdt.h"
#include "graphics.h"
#include <stddef.h>

static IDTEntry idt[256];
static IDTPointer idt_ptr;
static InterruptHandler handler_table[256];

const char *exception_messages[] = {"DIVISION BY ZERO",
                                    "DEBUG",
                                    "NON MASKABLE INTERRUPT",
                                    "BREAKPOINT",
                                    "INTO DETECTED OVERFLOW",
                                    "OUT OF BOUNDS",
                                    "INVALID OPCODE",
                                    "NO COPROCESSOR",
                                    "DOUBLE FAULT",
                                    "COPROCESSOR SEGMENT OVERRUN",
                                    "BAD TSS",
                                    "SEGMENT NOT PRESENT",
                                    "STACK FAULT",
                                    "GENERAL PROTECTION FAULT",
                                    "PAGE FAULT",
                                    "UNKNOWN INTERRUPT",
                                    "CO-PROCESSOR FAULT",
                                    "ALIGNMENT CHECK",
                                    "MACHINE CHECK",
                                    "SIMD FLOATING POINT EXCEPTION",
                                    "VIRTUALIZATION EXCEPTION",
                                    "CONTROL PROTECTION EXCEPTION", // 21
                                    "RESERVED",
                                    "RESERVED",
                                    "RESERVED",
                                    "RESERVED",
                                    "RESERVED",
                                    "RESERVED",
                                    "HYPervisor INJECTION EXCEPTION", // 28
                                    "VMX COMMUNICATION EXCEPTION",    // 29
                                    "SECURITY EXCEPTION",             // 30
                                    "RESERVED"};

extern void isr0();
extern void isr1();
extern void isr2();
extern void isr3();
extern void isr4();
extern void isr5();
extern void isr6();
extern void isr7();
extern void isr8();
extern void isr9();
extern void isr10();
extern void isr11();
extern void isr12();
extern void isr13();
extern void isr14();
extern void isr15();
extern void isr16();
extern void isr17();
extern void isr18();
extern void isr19();
extern void isr20();
extern void isr21();
extern void isr22();
extern void isr23();
extern void isr24();
extern void isr25();
extern void isr26();
extern void isr27();
extern void isr28();
extern void isr29();
extern void isr30();
extern void isr31();
extern void isr33(); // Vector 0x21
extern void isr64(); // Vector 0x40
extern void isr_generic();

void IDT_SetGate(uint8_t vector, void *handler, uint16_t selector,
                 uint8_t type_attr) {
  uintptr_t addr = (uintptr_t)handler;
  idt[vector].offset_low = addr & 0xFFFF;
  idt[vector].selector = selector;
  idt[vector].ist = 0;
  idt[vector].type_attr = type_attr;
  idt[vector].offset_mid = (addr >> 16) & 0xFFFF;
  idt[vector].offset_high = (addr >> 32) & 0xFFFFFFFF;
  idt[vector].zero = 0;
}

void Interrupt_RegisterHandler(uint8_t vector, InterruptHandler handler) {
  handler_table[vector] = handler;
}

uintptr_t ExceptionHandler(InterruptFrame *frame) {
  if (handler_table[frame->int_no]) {
    InterruptFrame *f = frame;
    handler_table[frame->int_no](&f);
    return (uintptr_t)f;
  }

  // Graphics_Clear(0x3B5998); // Blue screenish (Disabled to see debug logs)
  Graphics_Print(100, 100, "EXCEPTION OCCURRED!", 0xFFFFFF);
  Graphics_Print(100, 130, "INTERRUPT: ", 0xFFFFFF);
  if (frame->int_no < 32) {
    Graphics_Print(250, 130, exception_messages[frame->int_no], 0xFFFFFF);
    if (frame->int_no == 14) { // Page Fault
      uint64_t cr2;
      asm volatile("mov %%cr2, %0" : "=r"(cr2));
      Graphics_Print(100, 280, "CR2 (ADDR): ", 0xDC322F);
      Graphics_PrintHex(250, 280, cr2, 0xDC322F);
    }
  } else {
    Graphics_PrintHex(250, 130, frame->int_no, 0xFFFFFF);
  }

  Graphics_Print(100, 160, "ERROR CODE: ", 0xFFFFFF);
  Graphics_PrintHex(250, 160, frame->err_code, 0xFFFFFF);
  Graphics_Print(100, 190, "RIP: ", 0xFFFFFF);
  Graphics_PrintHex(250, 190, frame->rip, 0xFFFFFF);
  Graphics_Print(100, 220, "RAX: ", 0xFFFFFF);
  Graphics_PrintHex(250, 220, frame->rax, 0xFFFFFF);
  Graphics_Print(100, 250, "RSP: ", 0xFFFFFF);
  Graphics_PrintHex(250, 250, frame->rsp, 0xFFFFFF);

  while (1)
    asm("hlt");

  return (uintptr_t)frame;
}

// Assembly stubs
#define ISR_NOERR(n)                                                           \
  asm(".global isr" #n "\n"                                                    \
      "isr" #n ":\n"                                                           \
      "  pushq $0\n"                                                           \
      "  pushq $" #n "\n"                                                      \
      "  jmp isr_common\n");

#define ISR_ERR(n)                                                             \
  asm(".global isr" #n "\n"                                                    \
      "isr" #n ":\n"                                                           \
      "  pushq $" #n "\n"                                                      \
      "  jmp isr_common\n");

ISR_NOERR(0)
ISR_NOERR(1)
ISR_NOERR(2)
ISR_NOERR(3)
ISR_NOERR(4)
ISR_NOERR(5)
ISR_NOERR(6)
ISR_NOERR(7)
ISR_ERR(8)
ISR_NOERR(9)
ISR_ERR(10)
ISR_ERR(11)
ISR_ERR(12)
ISR_ERR(13)
ISR_ERR(14)
ISR_NOERR(15)
ISR_NOERR(16)
ISR_ERR(17)
ISR_NOERR(18)
ISR_NOERR(19)
ISR_NOERR(20)
ISR_ERR(21)
ISR_NOERR(22)
ISR_NOERR(23)
ISR_NOERR(24)
ISR_NOERR(25)
ISR_NOERR(26)
ISR_NOERR(27)
ISR_NOERR(28)
ISR_ERR(29)
ISR_ERR(30)
ISR_NOERR(31)
ISR_NOERR(33) // Vector 0x21

ISR_NOERR(64) // Vector 0x40

asm(".global isr_generic\n"
    "isr_generic:\n"
    "  pushq $0\n"
    "  pushq $255\n" // Generic flag
    "  jmp isr_common\n");

asm("isr_common:\n"
    "  pushq %rax\n"
    "  pushq %rbx\n"
    "  pushq %rcx\n"
    "  pushq %rdx\n"
    "  pushq %rbp\n"
    "  pushq %rdi\n"
    "  pushq %rsi\n"
    "  pushq %r8\n"
    "  pushq %r9\n"
    "  pushq %r10\n"
    "  pushq %r11\n"
    "  pushq %r12\n"
    "  pushq %r13\n"
    "  pushq %r14\n"
    "  pushq %r15\n"
    "  movq %rsp, %rcx\n" // First argument for Windows x64 ABI is RCX
    "  movq %rsp, %rbp\n" // Save RSP
    "  andq $-16, %rsp\n" // 16-byte align
    "  subq $32, %rsp\n"  // Shadow space for Windows ABI
    "  call ExceptionHandler\n"
    "  movq %rax, %rsp\n" // Use return value as new RSP
    "\n.global isr_restore\n"
    "isr_restore:\n"
    "  popq %r15\n"
    "  popq %r14\n"
    "  popq %r13\n"
    "  popq %r12\n"
    "  popq %r11\n"
    "  popq %r10\n"
    "  popq %r9\n"
    "  popq %r8\n"
    "  popq %rsi\n"
    "  popq %rdi\n"
    "  popq %rbp\n"
    "  popq %rdx\n"
    "  popq %rcx\n"
    "  popq %rbx\n"
    "  popq %rax\n"
    "  addq $16, %rsp\n" // Clean up int_no and err_code
    "  iretq\n");

void IDT_Init() {
  for (int i = 0; i < 256; i++) {
    IDT_SetGate(i, isr_generic, KERNEL_CODE_SEL, 0x8E);
    handler_table[i] = NULL;
  }

  // Type 0x8E = 1000 1110 (Present, DPL 0, Interrupt Gate)
  IDT_SetGate(0, isr0, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(1, isr1, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(2, isr2, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(3, isr3, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(4, isr4, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(5, isr5, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(6, isr6, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(7, isr7, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(8, isr8, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(9, isr9, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(10, isr10, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(11, isr11, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(12, isr12, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(13, isr13, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(14, isr14, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(15, isr15, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(16, isr16, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(17, isr17, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(18, isr18, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(19, isr19, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(20, isr20, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(21, isr21, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(22, isr22, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(23, isr23, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(24, isr24, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(25, isr25, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(26, isr26, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(27, isr27, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(28, isr28, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(29, isr29, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(30, isr30, KERNEL_CODE_SEL, 0x8E);
  IDT_SetGate(31, isr31, KERNEL_CODE_SEL, 0x8E);

  IDT_SetGate(33, isr33, KERNEL_CODE_SEL, 0x8E); // Vector 0x21 (Keyboard)
  IDT_SetGate(64, isr64, KERNEL_CODE_SEL, 0x8E); // Vector 0x40 (Timer)

  idt_ptr.limit = sizeof(idt) - 1;
  idt_ptr.base = (uint64_t)&idt;

  asm volatile("lidt %0" : : "m"(idt_ptr));
}

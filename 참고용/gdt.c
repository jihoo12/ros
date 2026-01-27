#include "gdt.h"
#include "libc.h" // For memset

static GDTEntry gdt[7];
static GDTPointer gdt_ptr;
TSS tss;

void GDT_SetEntry(int index, uint32_t base, uint32_t limit, uint8_t access,
                  uint8_t gran) {
  gdt[index].base_low = (base & 0xFFFF);
  gdt[index].base_middle = (base >> 16) & 0xFF;
  gdt[index].base_high = (base >> 24) & 0xFF;

  gdt[index].limit_low = (limit & 0xFFFF);
  gdt[index].granularity = (limit >> 16) & 0x0F;

  gdt[index].granularity |= gran & 0xF0;
  gdt[index].access = access;
}

// System Entry is 16 bytes for 64-bit TSS
void GDT_SetSystemEntry(int index, uint64_t base, uint32_t limit,
                        uint8_t access, uint8_t gran) {
  GDT_SetEntry(index, (uint32_t)base, limit, access, gran);

  // High part of the 64-bit descriptor
  struct SystemSegmentHigh {
    uint32_t base_high;
    uint32_t reserved;
  } __attribute__((packed)) *high = (void *)&gdt[index + 1];

  high->base_high = (uint32_t)(base >> 32);
  high->reserved = 0;
}

void TSS_SetStack(uint64_t kstack) { tss.rsp0 = kstack; }

void GDT_Init() {
  // Clear TSS
  memset(&tss, 0, sizeof(TSS));
  tss.iomap_base = sizeof(TSS);

  // Null descriptor
  GDT_SetEntry(0, 0, 0, 0, 0);

  // Kernel Code Segment: Access 0x9A, Granularity 0xAF (64-bit)
  GDT_SetEntry(1, 0, 0xFFFFFFFF, 0x9A, 0xAF);

  // Kernel Data Segment: Access 0x92, Granularity 0xCF
  GDT_SetEntry(2, 0, 0xFFFFFFFF, 0x92, 0xCF);

  // User Data Segment: Access 0xF2 (Present, Ring 3, Data, Writable)
  GDT_SetEntry(3, 0, 0xFFFFFFFF, 0xF2, 0xCF);

  // User Code Segment: Access 0xFA (Present, Ring 3, Code, Readable)
  GDT_SetEntry(4, 0, 0xFFFFFFFF, 0xFA, 0xAF);

  // TSS Segment: Access 0x89 (Present, Ring 0, Available TSS)
  GDT_SetSystemEntry(5, (uint64_t)&tss, sizeof(TSS) - 1, 0x89, 0x00);

  gdt_ptr.limit = sizeof(gdt) - 1;
  gdt_ptr.base = (uint64_t)&gdt;

  // Load GDT and Update Segments
  asm volatile("lgdt %0\n\t"
               "pushq %1\n\t"
               "leaq 1f(%%rip), %%rax\n\t"
               "pushq %%rax\n\t"
               "lretq\n\t"
               "1:\n\t"
               "mov %2, %%ds\n\t"
               "mov %2, %%es\n\t"
               "mov %2, %%fs\n\t"
               "mov %2, %%gs\n\t"
               "mov %2, %%ss\n\t"
               :
               : "m"(gdt_ptr), "i"(KERNEL_CODE_SEL), "r"(KERNEL_DATA_SEL)
               : "rax", "memory");

  // Load Task Register
  asm volatile("ltr %%ax" : : "a"(TSS_SEL));
}

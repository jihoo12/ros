#ifndef GDT_H
#define GDT_H

#include <stdint.h>

typedef struct {
  uint16_t limit_low;
  uint16_t base_low;
  uint8_t base_middle;
  uint8_t access;
  uint8_t granularity;
  uint8_t base_high;
} __attribute__((packed)) GDTEntry;

typedef struct {
  uint16_t limit;
  uint64_t base;
} __attribute__((packed)) GDTPointer;

typedef struct {
  uint32_t reserved1;
  uint64_t rsp0;
  uint64_t rsp1;
  uint64_t rsp2;
  uint64_t reserved2;
  uint64_t ist1;
  uint64_t ist2;
  uint64_t ist3;
  uint64_t ist4;
  uint64_t ist5;
  uint64_t ist6;
  uint64_t ist7;
  uint64_t reserved3;
  uint16_t reserved4;
  uint16_t iomap_base;
} __attribute__((packed)) TSS;

#define KERNEL_CODE_SEL 0x08
#define KERNEL_DATA_SEL 0x10
#define USER_DATA_SEL (0x18 | 3)
#define USER_CODE_SEL (0x20 | 3)
#define TSS_SEL 0x28

void GDT_Init();
void TSS_SetStack(uint64_t kstack);
extern TSS tss;

#endif

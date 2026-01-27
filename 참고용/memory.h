#ifndef MEMORY_H
#define MEMORY_H

#include "efi.h"
#define PAGE_SIZE 4096
#define MAX_PAGES                                                              \
  (1024 * 1024) // Support up to 4GB for now (1024*1024 * 4KB = 4GB)
extern uint8_t bitmap[MAX_PAGES / 8];
extern uint64_t total_pages;

void PageAllocator_Init(EFI_MEMORY_DESCRIPTOR *map, UINTN map_size,
                        UINTN desc_size);
void PageAllocator_MarkUsed(void *ptr, UINTN pages);
void *PageAllocator_Alloc(UINTN pages);
void PageAllocator_Free(void *ptr, UINTN pages);

// 4-level Page Table
typedef struct {
  uint64_t entries[512];
} PageTable;

#define PAGE_PRESENT (1ULL << 0)
#define PAGE_WRITABLE (1ULL << 1)
#define PAGE_USER (1ULL << 2)

void PageTable_Init(void *kernel_base, uint64_t kernel_size, void *fb_base,
                    uint64_t fb_size, EFI_MEMORY_DESCRIPTOR *map,
                    UINTN map_size, UINTN desc_size, uint64_t lapic_addr);
void PageTable_Map(PageTable *pml4, void *virt, void *phys, uint64_t flags);
void PageTable_UnMap(PageTable *pml4, void *virt);
void Memory_MapMMIO(void *phys_addr, uint64_t size);

#include "heap.h"

#endif

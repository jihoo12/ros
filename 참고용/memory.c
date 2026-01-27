#include "memory.h"
#include <stddef.h>

uint8_t bitmap[MAX_PAGES / 8];
uint64_t total_pages = 0;
PageTable *g_kernel_pml4 = NULL;

void PageAllocator_Init(EFI_MEMORY_DESCRIPTOR *map, UINTN map_size,
                        UINTN desc_size) {
  // Clear bitmap (mark all as used initially)
  for (int i = 0; i < sizeof(bitmap); i++)
    bitmap[i] = 0xFF;

  EFI_MEMORY_DESCRIPTOR *entry = map;
  void *map_end = (void *)((uint8_t *)map + map_size);

  while ((void *)entry < map_end) {
    uint64_t last_page = (entry->PhysicalStart +
                          entry->NumberOfPages * PAGE_SIZE + PAGE_SIZE - 1) /
                         PAGE_SIZE;
    if (last_page > total_pages && last_page < MAX_PAGES) {
      total_pages = last_page;
    }

    if (entry->Type == EfiConventionalMemory ||
        entry->Type == EfiBootServicesCode ||
        entry->Type == EfiBootServicesData) {
      uint64_t start_page = entry->PhysicalStart / PAGE_SIZE;
      uint64_t num_pages = entry->NumberOfPages;

      for (uint64_t i = 0; i < num_pages; i++) {
        uint64_t page = start_page + i;
        if (page < MAX_PAGES) {
          bitmap[page / 8] &= ~(1 << (page % 8));
        }
      }
    }
    entry = (EFI_MEMORY_DESCRIPTOR *)((uint8_t *)entry + desc_size);
  }
}

void PageAllocator_MarkUsed(void *ptr, UINTN pages) {
  uint64_t start_page = (uintptr_t)ptr / PAGE_SIZE;
  for (uint64_t i = 0; i < pages; i++) {
    uint64_t page = start_page + i;
    if (page < MAX_PAGES) {
      bitmap[page / 8] |= (1 << (page % 8));
    }
  }
}

void *PageAllocator_Alloc(UINTN pages) {
  if (pages == 0 || pages > total_pages)
    return NULL;

  for (uint64_t i = 1; i <= total_pages - pages; i++) {
    uint64_t found = 0;
    for (uint64_t j = 0; j < pages; j++) {
      if (bitmap[(i + j) / 8] & (1 << ((i + j) % 8))) {
        break;
      }
      found++;
    }

    if (found == pages) {
      // Mark as used
      for (uint64_t j = 0; j < pages; j++) {
        bitmap[(i + j) / 8] |= (1 << ((i + j) % 8));
      }
      return (void *)(i * PAGE_SIZE);
    }
  }

  return NULL;
}

void PageAllocator_Free(void *ptr, UINTN pages) {
  uint64_t start_page = (uintptr_t)ptr / PAGE_SIZE;
  for (uint64_t i = 0; i < pages; i++) {
    uint64_t page = start_page + i;
    if (page < MAX_PAGES) {
      bitmap[page / 8] &= ~(1 << (page % 8));
    }
  }
}

static PageTable *GetOrCreateTable(PageTable *table, int index) {
  if (table->entries[index] & PAGE_PRESENT) {
    return (PageTable *)(table->entries[index] & ~0xFFFULL);
  }

  PageTable *new_table = (PageTable *)PageAllocator_Alloc(1);
  if (!new_table)
    return NULL;

  // Clear new table
  for (int i = 0; i < 512; i++) {
    new_table->entries[i] = 0;
  }

  // Allow User Access in intermediate tables
  table->entries[index] =
      (uint64_t)new_table | PAGE_PRESENT | PAGE_WRITABLE | PAGE_USER;
  return new_table;
}

void PageTable_Map(PageTable *pml4, void *virt, void *phys, uint64_t flags) {
  uint64_t v = (uint64_t)virt;
  int pml4_idx = (v >> 39) & 0x1FF;
  int pdp_idx = (v >> 30) & 0x1FF;
  int pd_idx = (v >> 21) & 0x1FF;
  int pt_idx = (v >> 12) & 0x1FF;

  PageTable *pdp = GetOrCreateTable(pml4, pml4_idx);
  PageTable *pd = GetOrCreateTable(pdp, pdp_idx);
  PageTable *pt = GetOrCreateTable(pd, pd_idx);

  pt->entries[pt_idx] = (uint64_t)phys | flags | PAGE_PRESENT;
}

void PageTable_UnMap(PageTable *pml4, void *virt) {
  uint64_t v = (uint64_t)virt;
  int pml4_idx = (v >> 39) & 0x1FF;
  int pdp_idx = (v >> 30) & 0x1FF;
  int pd_idx = (v >> 21) & 0x1FF;
  int pt_idx = (v >> 12) & 0x1FF;

  if (!(pml4->entries[pml4_idx] & PAGE_PRESENT))
    return;
  PageTable *pdp = (PageTable *)(pml4->entries[pml4_idx] & ~0xFFFULL);

  if (!(pdp->entries[pdp_idx] & PAGE_PRESENT))
    return;
  PageTable *pd = (PageTable *)(pdp->entries[pdp_idx] & ~0xFFFULL);

  if (!(pd->entries[pd_idx] & PAGE_PRESENT))
    return;
  PageTable *pt = (PageTable *)(pd->entries[pd_idx] & ~0xFFFULL);

  pt->entries[pt_idx] = 0;

  // Invalidate TLB
  asm volatile("invlpg (%0)" : : "r"(virt) : "memory");
}

void PageTable_Init(void *kernel_base, uint64_t kernel_size, void *fb_base,
                    uint64_t fb_size, EFI_MEMORY_DESCRIPTOR *map,
                    UINTN map_size, UINTN desc_size, uint64_t lapic_addr) {
  PageTable *pml4 = (PageTable *)PageAllocator_Alloc(1);
  g_kernel_pml4 = pml4;
  for (int i = 0; i < 512; i++)
    pml4->entries[i] = 0;

  // 1. Map all usable physical memory (Identity Mapping)
  EFI_MEMORY_DESCRIPTOR *entry = map;
  void *map_end = (void *)((uint8_t *)map + map_size);

  while ((void *)entry < map_end) {
    if (entry->Type == EfiConventionalMemory || entry->Type == EfiLoaderCode ||
        entry->Type == EfiLoaderData || entry->Type == EfiBootServicesCode ||
        entry->Type == EfiBootServicesData ||
        entry->Type == EfiRuntimeServicesCode ||
        entry->Type == EfiRuntimeServicesData ||
        entry->Type == EfiACPIReclaimMemory ||
        entry->Type == EfiACPIMemoryNVS || entry->Type == EfiMemoryMappedIO ||
        entry->Type == EfiMemoryMappedIOPortSpace) {
      for (uint64_t addr = entry->PhysicalStart;
           addr < entry->PhysicalStart + entry->NumberOfPages * PAGE_SIZE;
           addr += PAGE_SIZE) {
        // Map with User permissions
        PageTable_Map(pml4, (void *)addr, (void *)addr,
                      PAGE_WRITABLE | PAGE_USER);
      }
    }
    entry = (EFI_MEMORY_DESCRIPTOR *)((uint8_t *)entry + desc_size);
  }

  // 2. Map Framebuffer
  for (uint64_t addr = (uint64_t)fb_base; addr < (uint64_t)fb_base + fb_size;
       addr += PAGE_SIZE) {
    PageTable_Map(pml4, (void *)addr, (void *)addr, PAGE_WRITABLE | PAGE_USER);
  }

  // 3. Map LAPIC and IOAPIC (usually 0xFEC00000)
  if (lapic_addr) {
    PageTable_Map(pml4, (void *)lapic_addr, (void *)lapic_addr, PAGE_WRITABLE);
  }
  // Common IOAPIC range
  for (uint64_t addr = 0xFEC00000; addr < 0xFEC01000; addr += PAGE_SIZE) {
    PageTable_Map(pml4, (void *)addr, (void *)addr, PAGE_WRITABLE);
  }

  // 4. Map Stack (rough estimate, 1MB around current RSP)
  uint64_t rsp;
  asm volatile("mov %%rsp, %0" : "=r"(rsp));
  for (uint64_t addr = (rsp & ~0xFFFFFULL);
       addr < (rsp & ~0xFFFFFULL) + 0x100000; addr += PAGE_SIZE) {
    PageTable_Map(pml4, (void *)addr, (void *)addr, PAGE_WRITABLE | PAGE_USER);
  }

  // Load PML4 into CR3
  asm volatile("mov %0, %%cr3" : : "r"(pml4) : "memory");
}

void Memory_MapMMIO(void *phys_addr, uint64_t size) {
  if (!g_kernel_pml4)
    return;
  for (uint64_t addr = (uint64_t)phys_addr; addr < (uint64_t)phys_addr + size;
       addr += PAGE_SIZE) {
    // Map as Writable | User (so ring 3 can also access if needed, or just
    // Writable)
    PageTable_Map(g_kernel_pml4, (void *)addr, (void *)addr,
                  PAGE_WRITABLE | PAGE_USER);
    asm volatile("invlpg (%0)" : : "r"(addr) : "memory");
  }
}

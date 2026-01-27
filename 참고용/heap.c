#include "heap.h"
#include <stddef.h>

static HeapBlock *free_list = NULL;

void Heap_Init(void *start, size_t size) {
  free_list = (HeapBlock *)start;
  free_list->size = size - sizeof(HeapBlock);
  free_list->next = NULL;
  free_list->prev = NULL;
  free_list->free = 1;
}

void *kmalloc(size_t size) {
  HeapBlock *current = free_list;

  // Simple First-Fit
  while (current) {
    if (current->free && current->size >= size) {
      // Can we split the block?
      if (current->size >= size + sizeof(HeapBlock) + 16) {
        HeapBlock *next =
            (HeapBlock *)((uint8_t *)current + sizeof(HeapBlock) + size);
        next->size = current->size - size - sizeof(HeapBlock);
        next->next = current->next;
        next->prev = current;
        next->free = 1;

        if (next->next)
          next->next->prev = next;

        current->size = size;
        current->next = next;
      }

      current->free = 0;
      return (void *)((uint8_t *)current + sizeof(HeapBlock));
    }
    current = current->next;
  }

  return NULL;
}

void *kmalloc_aligned(size_t size, size_t alignment) {
  if (alignment <= 16) {
    return kmalloc(size);
  }

  // Over-allocate to ensure we can find an aligned address within the block
  // We need enough space for:
  // 1. The requested size
  // 2. The alignment padding (up to alignment - 1)
  // 3. A new HeapBlock header for the aligned block
  size_t total_size = size + alignment + sizeof(HeapBlock);
  void *ptr = kmalloc(total_size);
  if (!ptr)
    return NULL;

  uintptr_t raw_addr = (uintptr_t)ptr;
  uintptr_t aligned_addr =
      (raw_addr + sizeof(HeapBlock) + alignment - 1) & ~(alignment - 1);

  // We need to place a header right before aligned_addr
  HeapBlock *original_block = (HeapBlock *)((uint8_t *)ptr - sizeof(HeapBlock));
  HeapBlock *aligned_block = (HeapBlock *)(aligned_addr - sizeof(HeapBlock));

  if (aligned_block != original_block) {
    // Split the block: [original_block (padding)] -> [aligned_block (data)]
    aligned_block->size = original_block->size - ((uintptr_t)aligned_block -
                                                  (uintptr_t)original_block);
    aligned_block->next = original_block->next;
    aligned_block->prev = original_block;
    aligned_block->free = 0;

    if (aligned_block->next)
      aligned_block->next->prev = aligned_block;

    original_block->size = (uintptr_t)aligned_block -
                           (uintptr_t)original_block - sizeof(HeapBlock);
    original_block->next = aligned_block;
    original_block->free = 1; // The padding is now free!

    // Coalesce the padding if possible (optional but good)
    kfree((void *)((uint8_t *)original_block + sizeof(HeapBlock)));
  }

  return (void *)aligned_addr;
}

void kfree(void *ptr) {
  if (!ptr)
    return;

  HeapBlock *block = (HeapBlock *)((uint8_t *)ptr - sizeof(HeapBlock));
  block->free = 1;

  // Coalesce with next
  if (block->next && block->next->free) {
    block->size += block->next->size + sizeof(HeapBlock);
    block->next = block->next->next;
    if (block->next)
      block->next->prev = block;
  }

  // Coalesce with prev
  if (block->prev && block->prev->free) {
    block->prev->size += block->size + sizeof(HeapBlock);
    block->prev->next = block->next;
    if (block->next)
      block->next->prev = block->prev;
  }
}

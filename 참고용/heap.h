#ifndef HEAP_H
#define HEAP_H

#include <stddef.h>
#include <stdint.h>

typedef struct HeapBlock {
  size_t size;
  struct HeapBlock *next;
  struct HeapBlock *prev;
  int free;
} HeapBlock;

void Heap_Init(void *start, size_t size);
void *kmalloc(size_t size);
void *kmalloc_aligned(size_t size, size_t alignment);
void kfree(void *ptr);

#endif

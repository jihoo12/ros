#include "syscall.h"
#include "gdt.h"
#include "graphics.h"
#include "interrupt.h"
#include "memory.h"
#include "nvme.h"
#include "schedule.h"
#include <stdint.h>

extern void syscall_entry();

void MSR_Write(uint32_t msr, uint64_t val) {
  uint32_t low = val & 0xFFFFFFFF;
  uint32_t high = val >> 32;
  asm volatile("wrmsr" : : "a"(low), "d"(high), "c"(msr));
}

uint64_t MSR_Read(uint32_t msr) {
  uint32_t low, high;
  asm volatile("rdmsr" : "=a"(low), "=d"(high) : "c"(msr));
  return ((uint64_t)high << 32) | low;
}

// Per-CPU data structure for GS segment
typedef struct {
  uint64_t user_rsp_scratch; // Offset 0
  uint64_t kernel_stack;     // Offset 8 (Optional usage)
} CpuData;

void Syscall_Init() {
  // 0. Setup Per-CPU Data for GS
  CpuData *cpu_data = (CpuData *)PageAllocator_Alloc(1); // 1 Page
  if (cpu_data) {
    // Write the address of the structure to MSR_KERNEL_GS_BASE
    // When executing 'swapgs' in syscall_entry, GS Base will point here.
    MSR_Write(MSR_KERNEL_GS_BASE, (uint64_t)cpu_data);
  }

  // 1. Enable SCE (System Call Extensions) in EFER
  uint64_t efer = MSR_Read(MSR_EFER);
  efer |= EFER_SCE;
  MSR_Write(MSR_EFER, efer);

  // 2. Set STAR (0xC0000081)
  // Bits 63:48 = User CS Base (sysret CS) -> User CS is 0x20.
  // SYSRET loads CS = STAR[63:48] + 16 = 0x20 + 0x10 = 0x30?
  // Wait.
  // SYSRET (64-bit):
  //   CS = STAR[63:48] + 16 (selector) -> If we put 0x13 in high.
  //   SS = STAR[63:48] + 8 (selector)
  // We have USER_CODE_SEL = 0x20 | 3. USER_DATA_SEL = 0x18 | 3.
  // The selectors must be consecutive.
  // If STAR[63:48] = 0x13 (User Code Base index?), let's verify.
  // GDT: 0=Null, 1=KCode, 2=KData, 3=UData(0x18), 4=UCode(0x20), 5=TSS.
  // Linux GDT:
  // KCode(0), KData(1)...
  //
  // Intel SDM:
  // SYSRET CS selector = STAR[63:48] + 16.
  // SYSRET SS selector = STAR[63:48] + 8.
  // If we want CS=0x23 (Index 4, RPL 3), SS=0x1B (Index 3, RPL 3).
  // Then:
  // STAR[63:48]+16 = 0x20 (Base selector is 0x20? Or we provide Base 0x10?)
  //
  // If STAR[63:48] = 0x10 (Index 2).
  // CS = 0x10 + 16 = 0x20 (Index 4). Correct.
  // SS = 0x10 + 8  = 0x18 (Index 3). Correct.
  // So STAR[63:48] should be 0x13 (RPL 3)? Or just 0x10?
  // It loads the selector with forced RPL 3.
  // So we set STAR[63:48] = 0x10 | 3 ? No, just base.
  // "The specific numeric value is loaded... with RPL forced to 3."
  // So we put 0x10.
  //
  // Bits 47:32 = Kernel CS Base (syscall CS).
  // SYSCALL loads CS = STAR[47:32].
  // SYSCALL loads SS = STAR[47:32] + 8.
  // We want KERNEL_CODE(0x08) and KERNEL_DATA(0x10).
  // So STAR[47:32] = 0x08.
  //
  // Result:
  // High = (0x10 | 3) << 16 | (0x08).
  // Actually, usually we pass (USER_CS_BASE_SEL << 16) | KERNEL_CS_SEL in the
  // high uint32 of MSR. But USER_CS_BASE_SEL logic is tricky: CS = Base + 16.
  // We want 0x20. So Base=0x10 (User Data Sel). SS = Base + 8.  We want 0x18.
  // So Base=0x10. So Base = 0x0010.

  uint64_t star = ((uint64_t)0x0013 << 48) | ((uint64_t)0x0008 << 32);
  // Wait, if I use 0x13 (User code is index 4, User data index 3).
  // Target: CS=Index 4, SS=Index 3.
  // Rule:
  // CS = STAR[63:48] + 16
  // SS = STAR[63:48] + 8
  // If STAR[63:48] = X.
  // Index 4 = X + 16 => X = Index 4 - 16?? No, indices are bytes.
  // 0x20 = X + 0x10 => X = 0x10.
  // 0x18 = X + 0x08 => X = 0x10.
  // So X = 0x10 (Kernel Data Selector).
  // BUT: "The CPL is set to 3".
  // Linux uses 0x13?
  // Let's use 0x10 | 3 = 0x13.
  // Actually, the selector loaded is (Star[63:48]+16) | 3.
  // So if I put 0x10, it loads 0x20 | 3 = 0x23.
  // If I put 0x13, it loads 0x23 | 3 = 0x23.
  // Let's stick to 0x13 (with RPL bits) to be safe/explicit if expected, but
  // standard is just the segment index. Let's use 0x0013.

  // star = ((uint64_t)USER_DATA_SEL << 48) | ((uint64_t)KERNEL_CODE_SEL << 32);
  // ??? USER_DATA_SEL is 0x1B (Index 3 | 3). base = 0x1B? CS = 0x1B + 0x10 =
  // 0x2B? No. We need 0x23. 0x23 - 0x10 = 0x13. So 0x13 is correct. And 0x13 -
  // 0x8 must match SS? No SS = Base + 8. 0x13 + 8 = 0x1B. So 0x13 is the magic
  // number.

  star = ((uint64_t)0x0013 << 48) | ((uint64_t)KERNEL_CODE_SEL << 32);
  MSR_Write(MSR_STAR, star);

  // 3. Set LSTAR (Target RIP)
  MSR_Write(MSR_LSTAR, (uint64_t)syscall_entry);

  // 4. Set SFMASK (RFLAGS mask)
  // Mask Interrupts (IF=0x200)
  MSR_Write(MSR_SFMASK, 0x200);
}

static uint32_t console_x = 10;
static uint32_t console_y = 10;

uint64_t Syscall_Handler(uint64_t sys_num, uint64_t a1, uint64_t a2,
                         uint64_t a3, uint64_t a4, uint64_t a5) {
  switch (sys_num) {
  case SYSCALL_CLEAR: {
    Graphics_Clear(a1);
    console_x = 10;
    console_y = 10;
    break;
  }
  case SYSCALL_PRINT: {
    // a1 = string pointer
    // a2 = length? Or null terminated.
    // a3 = color?
    char *str = (char *)a1;
    uint32_t color = (uint32_t)a2;

    uint32_t width = 0, height = 0;
    Graphics_GetDimensions(&width, &height);
    if (width == 0)
      width = 800; // Fallback
    if (height == 0)
      height = 600;

    while (*str) {
      char c = *str;
      if (c == '\n') {
        console_x = 10;
        console_y += 16;
      } else if (c == '\r') {
        console_x = 10;
      } else {
        Graphics_PutChar(console_x, console_y, c, color);
        console_x += 8;
        if (console_x >= width - 8) {
          console_x = 10;
          console_y += 16;
        }
      }

      if (console_y >= height - 16) {
        console_y = 10;
        Graphics_Clear(0x000000); // Clear screen on wrap for now
      }
      str++;
    }
    break;
  }
  case SYSCALL_EXEC: {
    // a1 = function pointer
    // a2 = stack pages count
    void *stack = PageAllocator_Alloc(a2);
    if (!stack) {
      break;
    }
    void (*func_ptr)() = (void (*)())a1;
    // Use Scheduler_AddUserTask to run as Ring 3 with correct stack management
    Scheduler_AddUserTask(func_ptr, stack, a2);

    break;
  }
  case SYSCALL_TERMINATE: {
    // Terminate current task and switch to next
    // We need to pass a pointer to a frame pointer, but we don't care about
    // saving current frame. However, Scheduler_TerminateCurrentTask updates the
    // *pointer* to the NEW frame.

    InterruptFrame *new_rsp = NULL;
    // We pass address of new_rsp. Scheduler will write the new task's RSP to
    // new_rsp.
    Scheduler_TerminateCurrentTask(&new_rsp);

    // Return the new RSP to assembly wrapper, which will switch stack and jump
    // to isr_restore
    return (uint64_t)new_rsp;
  }
  case SYSCALL_HALT: {
    asm volatile("hlt");
  }
  case SYSCALL_NVME_READ: {
    // a1 = nsid (uint32)
    // a2 = lba (uint64)
    // a3 = buffer (ptr)
    // a4 = count (uint32)
    NVMe_Read((uint32_t)a1, (uint64_t)a2, (void *)a3, (uint32_t)a4);
    break;
  }
  case SYSCALL_NVME_WRITE: {
    // a1 = nsid (uint32)
    // a2 = lba (uint64)
    // a3 = buffer (ptr)
    // a4 = count (uint32)
    NVMe_Write((uint32_t)a1, (uint64_t)a2, (void *)a3, (uint32_t)a4);
    break;
  }
  case SYSCALL_KMALLOC: {
    // a1 = size (uint64)
    void *ptr = kmalloc((size_t)a1);
    a1 = (uint64_t)ptr;
  }
  case SYSCALL_KFREE: {
    // a1 = ptr (void*)
    kfree((void *)a1);
    break;
  }
  default: {
    Graphics_Clear(0xEEE8D5);
    Graphics_Print(100, 100, "SYSCALL NOT IMPLEMENTED", 0x268BD2);
    break;
  }
  }
  return 0; // 0 means no context switch, return normally via sysretq
}

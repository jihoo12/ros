#![allow(static_mut_refs)]
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::arch::asm;
use core::sync::atomic::{AtomicUsize, Ordering};

// Re-using the allocator from the crate
use crate::allocator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Ready,
    Running,
    Terminated,
}

pub struct Task {
    pub id: usize,
    pub stack_top: u64,    // Saved Stack Pointer (RSP)
    pub stack_bottom: u64, // For deallocation reference
    pub status: TaskStatus,
}

pub struct Scheduler {
    tasks: Vec<Task>,
    current_task_index: usize,
}

static mut SCHEDULER: Option<Scheduler> = None;
static NEXT_TASK_ID: AtomicUsize = AtomicUsize::new(1); // 0 is reserved for main kernel task

/// Initialize the global scheduler.
/// This must be called only once.
pub unsafe fn init() {
    SCHEDULER = Some(Scheduler {
        tasks: Vec::new(),
        current_task_index: 0,
    });

    // Create a dummy task for the currently running kernel thread (Main Task)
    // We don't need to allocate a stack for it because it's already using one.
    // When we switch AWAY from it, we will save its state.
    let main_task = Task {
        id: 0,
        stack_top: 0,    // Will be updated on first switch
        stack_bottom: 0, // 0 implies we don't own this stack, so don't free it
        status: TaskStatus::Running,
    };

    if let Some(scheduler) = SCHEDULER.as_mut() {
        scheduler.tasks.push(main_task);
    }
}

pub fn add_new_task(entry_point: extern "C" fn(), stack_bottom: u64, stack_size: usize) {
    unsafe {
        if let Some(scheduler) = SCHEDULER.as_mut() {
            let id = NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst);

            // 2. Setup Stack Frame for Context Switch
            let stack_top = stack_bottom + stack_size as u64;

            // Stack grows DOWN.
            // Alignment Requirement: RSP + 8 must be 16-byte aligned.
            // So on ENTRY (instruction 0), RSP should be `...8`.
            // Our `stack_top` is 16-byte aligned (`...0`) usually.
            // So we should start filling from `stack_top - 8`.

            let mut sp = (stack_top - 8) as *mut u64;

            // Return Address (RIP) - This is where we jump when we switch TO this task
            sp = sp.sub(1);
            *sp = entry_point as u64; // RIP

            // RBP
            sp = sp.sub(1);
            *sp = 0; // Initial RBP

            // RBX
            sp = sp.sub(1);
            *sp = 0;

            // R12
            sp = sp.sub(1);
            *sp = 0;

            // R13
            sp = sp.sub(1);
            *sp = 0;

            // R14
            sp = sp.sub(1);
            *sp = 0;

            // R15
            sp = sp.sub(1);
            *sp = 0; // r15

            // 3. Create Task Struct
            let task = Task {
                id,
                stack_top: sp as u64, // The saved RSP
                stack_bottom,
                status: TaskStatus::Ready,
            };

            scheduler.tasks.push(task);
        }
    }
}

pub fn switch_task() {
    unsafe {
        if let Some(scheduler) = SCHEDULER.as_mut() {
            let current_index = scheduler.current_task_index;

            // Round-robin: find next Ready task
            let mut next_index = (current_index + 1) % scheduler.tasks.len();
            let mut found = false;

            // Loop once to find a ready task
            for _ in 0..scheduler.tasks.len() {
                if scheduler.tasks[next_index].status == TaskStatus::Ready {
                    found = true;
                    break;
                }
                // If current task is running, it's also a candidate (if it's the only one, or we just cycle back)
                // But wait, if we are currently Running, we should be marked Ready if we yield?
                // Or we just stay Running if it's round robin.

                // Let's adhere to: Only switch to Ready tasks.
                // The current task is "Running". If we yield, it becomes "Ready".

                next_index = (next_index + 1) % scheduler.tasks.len();
            }

            if !found {
                // If no other task is Ready, check if current is still runnable.
                if scheduler.tasks[current_index].status == TaskStatus::Terminated {
                    // We are terminated and no one else to run? deadlock/halt
                    crate::println!("All tasks could be terminated, or deadlock. Halting.");
                    loop {
                        core::arch::asm!("hlt");
                    }
                }
                // Just continue current task
                return;
            }

            if next_index == current_index {
                // No switch needed
                return;
            }

            // Update statuses
            let old_index = current_index;

            if scheduler.tasks[old_index].status == TaskStatus::Running {
                scheduler.tasks[old_index].status = TaskStatus::Ready;
            }

            scheduler.tasks[next_index].status = TaskStatus::Running;
            scheduler.current_task_index = next_index;

            let old_stack_ref = &mut scheduler.tasks[old_index].stack_top as *mut u64;
            let new_stack = scheduler.tasks[next_index].stack_top;

            // Perform the low-level switch
            context_switch(old_stack_ref, new_stack);
        }
    }
}

pub fn terminate_task() {
    unsafe {
        if let Some(scheduler) = SCHEDULER.as_mut() {
            let current_index = scheduler.current_task_index;
            scheduler.tasks[current_index].status = TaskStatus::Terminated;

            // Free stack?
            // If we free the stack NOW, we are still using it!
            // We cannot free the stack we are currently running on until we switch away.
            // Simplest for now: Don't free immediately, or have a "cleanup" task.
            // For this simple implementation, we LEAK the stack of the terminated task
            // OR we rely on the next task to clean it up?
            // Let's just mark Terminated and NOT free for this simple version.

            crate::println!("Task {} terminated.", scheduler.tasks[current_index].id);

            // Force switch
            switch_task();
        }
    }
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "sysv64" fn context_switch(old_stack_ptr: *mut u64, new_stack_ptr: u64) {
    core::arch::naked_asm!(
        "push r15",
        "push r14",
        "push r13",
        "push r12",
        "push rbx",
        "push rbp",
        // Save current RSP to the old_stack_ptr location
        "mov [rdi], rsp",
        // Load new RSP
        "mov rsp, rsi",
        "pop rbp",
        "pop rbx",
        "pop r12",
        "pop r13",
        "pop r14",
        "pop r15",
        "ret", // Jumps to return address on top of new stack
    );
}

// Helper to get current task id
pub fn current_task_id() -> usize {
    unsafe {
        if let Some(scheduler) = SCHEDULER.as_ref() {
            scheduler.tasks[scheduler.current_task_index].id
        } else {
            0
        }
    }
}

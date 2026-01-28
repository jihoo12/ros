#include "keyboard.h"
#include "apic.h"
#include "graphics.h"
#include "io.h"
#include "schedule.h"
#include "timer.h"
#include <stddef.h>

static char last_char = 0;

const char scancode_to_ascii[] = {
    0,    27,  '1', '2', '3',  '4', '5', '6', '7',  '8', /* 9 */
    '9',  '0', '-', '=', '\b',                           /* Backspace */
    '\t',                                                /* Tab */
    'q',  'w', 'e', 'r',                                 /* 19 */
    't',  'y', 'u', 'i', 'o',  'p', '[', ']', '\n',      /* Enter key */
    0,                                                   /* 29   - Control */
    'a',  's', 'd', 'f', 'g',  'h', 'j', 'k', 'l',  ';', /* 39 */
    '\'', '`', 0,                                        /* Left shift */
    '\\', 'z', 'x', 'c', 'v',  'b', 'n',                 /* 49 */
    'm',  ',', '.', '/', 0,                              /* Right shift */
    '*',  0,                                             /* Alt */
    ' ',                                                 /* Space bar */
    0,                                                   /* Caps lock */
    0,                                                   /* 59 - F1 key ... > */
    0,    0,   0,   0,   0,    0,   0,   0,   0,         /* < ... F10 */
    0,                                                   /* 69 - Num lock*/
    0,                                                   /* Scroll Lock */
    0,                                                   /* Home key */
    0,                                                   /* Up Arrow */
    0,                                                   /* Page Up */
    '-',  0,                                             /* Left Arrow */
    0,    0,                                             /* Right Arrow */
    '+',  0,                                             /* 79 - End key*/
    0,                                                   /* Down Arrow */
    0,                                                   /* Page Down */
    0,                                                   /* Insert Key */
    0,                                                   /* Delete Key */
    0,    0,   0,   0,                                   /* F11 Key */
    0,                                                   /* F12 Key */
    0, /* All other keys are undefined */
};

void Keyboard_Handler(InterruptFrame **frame_ptr) {
  uint8_t scancode = inb(0x60);
  LAPIC_SendEOI();

  if (scancode == 0x3A) { // CapsLock Make
    Scheduler_Switch(frame_ptr);
    return;
  }

  if (scancode == 0x01) { // ESC Key Make
    Scheduler_TerminateCurrentTask(frame_ptr);
    return;
  }

  if (scancode & 0x80) {
    // Key release
  } else {
    // Key press
    if (scancode < sizeof(scancode_to_ascii)) {
      last_char = scancode_to_ascii[scancode];
      if (last_char) {
        Graphics_PutChar(100 + (Timer_GetTicks() % 50) * 8, 550, last_char,
                         0xFFFFFF);
      }
    }
  }
}

char Keyboard_GetLastChar() {
  char c = last_char;
  last_char = 0;
  return c;
}

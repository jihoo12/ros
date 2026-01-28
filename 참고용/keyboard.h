#ifndef KEYBOARD_H
#define KEYBOARD_H

#include "interrupt.h"
#include <stdint.h>

#define KEYBOARD_IRQ 1
#define INT_KEYBOARD 0x21

void Keyboard_Handler(InterruptFrame **frame);
char Keyboard_GetLastChar();

#endif

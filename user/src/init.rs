#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

mod std;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let msg = "\n=========================================\n\
               🦀 Hello from Rust User Mode (Ring 3)! 🦀\n\
               =========================================\n\
               init.kef loaded and executed successfully.\n";
    std::print(msg);

    // Let's poll for keypress to shut down
    std::print("Press any key to trigger shutdown...\n");

    loop {
        std::poll_xhci();
        let key = std::read_key();
        if key != 0 {
            break;
        }
    }

    std::print("\nShutting down the system. Goodbye!\n");
    std::shutdown();
}
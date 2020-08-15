use core::panic::PanicInfo;

use crate::println;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(not(test))]
#[lang = "eh_personality"]
#[no_mangle]
pub extern "C" fn eh_personality() {}

use core::panic::PanicInfo;

use alloc::string::String;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    let loc = info.location().unwrap();
    match info.message() {
        Some(v) => println!("{}: Panic: '{}'", loc, v),
        None => {
            let msg = match info.payload().downcast_ref::<&'static str>() {
                Some(v) => *v,
                None => match info.payload().downcast_ref::<String>() {
                    Some(v) => &v[..],
                    None => "Box<Any>",
                },
            };
            println!("{}: Panic: '{}'", loc, msg);
        }
    }

    loop {}
}

#![no_std]
#![no_main]

use userspace_rs::library;

#[no_mangle]
pub extern "C" fn _start() {
    let mut data_buffer: [u8; 1024] = [0; 1024];
    let welcome = "Hello, type something!\n".as_bytes();
    let bullet = ">>> ".as_bytes();

    unsafe {
        library::syscalls::sys_write(1, &welcome, welcome.len());

        loop {
            library::syscalls::sys_write(1, &bullet, bullet.len());
            let read_length = library::syscalls::sys_read(0, &mut data_buffer, 1024);
            library::syscalls::sys_write(1, &data_buffer[0..read_length], read_length);
            for idx in 0..read_length {
                data_buffer[idx] = 0;
            }
        }
    }
}
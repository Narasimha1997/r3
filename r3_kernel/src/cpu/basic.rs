

pub fn halt() {
    unsafe {
        asm!("hlt");
    }
}
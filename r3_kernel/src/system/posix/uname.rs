use crate::mm::VirtualAddress;
use crate::system::abi;

// these strings are null terminated to make sure they are processed
// properly by usespace C libraries which uses null terminated strings.
const UNAME_STRINGS: &'static [&'static str] = &["LINUX", "", "0.0.1", "r3", "x86_64", ""];

const FIELD_LEN: usize = 65;

pub fn sys_uname(buffer_addr: VirtualAddress) -> Result<isize, abi::Errno> {
    unsafe {
        let mut buffer_ptr = buffer_addr.get_mut_ptr::<u8>();

        for string in UNAME_STRINGS {
            let result = abi::copy_pad_buffer(string.as_bytes(), buffer_ptr, string.len());
            if result.is_err() {
                return Err(result.unwrap_err());
            }

            buffer_ptr = buffer_ptr.add(FIELD_LEN);
        }

        Ok(0)
    }
}

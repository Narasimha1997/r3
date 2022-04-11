#![no_std]
#![no_main]

use core::fmt::Write;
use core::str;
use userspace_rs::library;

use library::utils::{get_uname, read_stdin, str_from_c_like_buffer};
use userspace_rs::{print, println};

#[inline]
fn echo(arg_str: &str) {
    println!("{}", arg_str);
}

#[inline]
fn uname(_arg_str: &str) {
    // TODO: Handle options
    let uname_res = get_uname();
    if let Ok(uname_struct) = uname_res {
        // print uname info
        unsafe {
            let sys_name = str_from_c_like_buffer(&uname_struct.sys_name);
            let node_name = str_from_c_like_buffer(&uname_struct.node_name);
            let release = str_from_c_like_buffer(&uname_struct.release);
            let version = str_from_c_like_buffer(&uname_struct.version);
            let machine = str_from_c_like_buffer(&uname_struct.machine);
            let domain = str_from_c_like_buffer(&uname_struct.domain);
            println!(
                "{} {} {} {} {} {}",
                sys_name, node_name, version, release, machine, domain
            );
        }
    } else {
        println!(
            "'uname' exited with invalid code: {}",
            uname_res.unwrap_err()
        );
    }
}

#[inline(always)]
fn get_string_view(buffer: &[u8], length: usize) -> &str {
    if let Ok(string) = str::from_utf8(&buffer[0..length]) {
        return string;
    } else {
        return "";
    }
}

fn exec_command(string: &str) {
    let command_str: &str;
    if let Some(end_index) = string.find(' ') {
        command_str = &string[0..end_index];
    } else {
        command_str = string;
    }

    match command_str {
        "uname" => {
            uname(&string[command_str.len()..string.len()]);
        }
        "echo" => {
            echo(&string[command_str.len()..string.len()]);
        }
        _ => {
            println!("unknown command {}", command_str);
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() {
    let mut data_buffer: [u8; 1024] = [0; 1024];

    loop {
        print!("[root@root]~# ");
        let read_length = read_stdin(&mut data_buffer, 1024);
        let str_view = get_string_view(&data_buffer, read_length - 1);
        exec_command(str_view);

        for idx in 0..read_length {
            data_buffer[idx] = 0;
        }
    }
}

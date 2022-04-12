#![no_std]
#![no_main]

use core::fmt::Write;
use core::str;
use userspace_rs::library;

use library::utils::{
    get_uname, read_stdin, str_from_c_like_buffer, power_off_machine,
    lstat,
};
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

#[inline]
fn shutdown(_arg_str: &str) {
    power_off_machine();
}

#[inline]
fn sizeof(arg_str: &str) {
    let lstat_result = lstat(&arg_str);
    if let Ok(lstat_buffer) = lstat_result {
        let f_size = lstat_buffer.file_size;
        println!("size of {}: {}bytes", arg_str, f_size);
    } else {
        let lstat_err_code = lstat_result.unwrap_err() as usize;
        println!(
            "'sizeof' exited with invalid code: {}",
            lstat_err_code
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
        command_str = string.trim();
    }

    let remaining_str = string[command_str.len()..string.len()].trim();

    match command_str {
        "uname" => {
            uname(&remaining_str);
        }
        "echo" => {
            echo(&remaining_str);
        }
        "shutdown" => {
            shutdown(&remaining_str);
        }
        "sizeof" => {
            sizeof(&remaining_str);
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

        // replace last character with null
        data_buffer[read_length - 1] = '\0' as u8;

        let str_view = get_string_view(&data_buffer, read_length - 1);
        exec_command(str_view);

        for idx in 0..read_length {
            data_buffer[idx] = 0;
        }
    }
}

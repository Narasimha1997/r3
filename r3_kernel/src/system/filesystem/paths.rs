extern crate alloc;

use alloc::{string::String, vec::Vec};

#[inline]
pub fn create_iter<'a>(path: &'a str) -> impl Iterator<Item = &'a str> {
    path.split("/")
}

pub fn resolve(path: &str) -> Option<String> {
    let mut path_items: Vec<String> = Vec::new();
    for path_item in create_iter(&path) {
        match path_item {
            "." => {}
            ".." => {
                path_items.pop();
            }
            _ => path_items.push(String::from(path_item)),
        }
    }

    Some(path_items.join("/"))
}

#[inline]
pub fn as_string(path: &str) -> String {
    String::from(path)
}

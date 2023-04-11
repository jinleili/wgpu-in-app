extern crate objc;
use self::objc::{runtime::Object, *};
extern crate objc_foundation;
use self::objc_foundation::{INSString, NSString};

extern crate lazy_static;
use self::lazy_static::*;

use std::path::PathBuf;

lazy_static! {
    static ref BUNDLE_PATH: &'static str = get_bundle_url();
}

fn get_bundle_url() -> &'static str {
    let cls = class!(NSBundle);
    let path: &str = unsafe {
        // Allocate an instance
        let bundle: *mut Object = msg_send![cls, mainBundle];
        // let url: *mut Object = msg_send![*bundle, resourcePath];
        // 资源路径要用 resourcePath
        let path: &NSString = msg_send![bundle, resourcePath];
        path.as_str()
    };
    path
}

pub struct FileSystem<'a> {
    _base_path: &'a str,
}

impl<'a> FileSystem<'a> {
    pub fn new(_base_path: &'a str) -> Self {
        FileSystem { _base_path }
    }

    pub fn get_bundle_url() -> &'static str {
        &BUNDLE_PATH
    }

    pub fn get_shader_path(name: &str, suffix: &str) -> String {
        FileSystem::get_spirv_file_path(name, suffix)
    }

    fn get_spirv_file_path(name: &str, suffix: &str) -> String {
        let mut p = name.to_string().replace('/', "_");
        p = get_bundle_url().to_string() + "/shader-spirv/" + &p;
        p += &format!("_{suffix}.spv");

        p
    }

    pub fn get_texture_file_path(&self, name: &str) -> PathBuf {
        let p = get_bundle_url().to_string() + "/assets/" + name;
        PathBuf::from(&p)
    }
}

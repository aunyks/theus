use std::ffi::CString;
use theus::c_compatible;

pub struct MyStruct {
    msg: CString,
    a: i32,
}

#[c_compatible]
impl MyStruct {
    pub fn create() -> Self {
        Self {
            msg: CString::new("hey there").unwrap(),
            a: 3,
        }
    }

    pub fn get_value(&mut self) -> i32 {
        0
    }

    pub fn get_cstring(&mut self) -> *const i8 {
        self.msg.as_ptr()
    }

    pub fn get_cstring_len(&mut self) -> usize {
        self.msg.as_bytes_with_nul().len()
    }

    pub fn set_cstring(&mut self, cstring_ptr: *mut i8) {
        self.msg = unsafe { CString::from_raw(cstring_ptr) };
    }

    pub fn set_slice(&mut self, sl: &mut [u8]) {}

    pub fn set_string(&mut self, str: &mut String) {}

    pub fn set_num(&mut self, n: i32) {}

    pub fn get_num(&mut self) -> String {
        "".to_owned()
    }

    pub fn get_str_raw(&mut self) -> &mut i32 {
        &mut self.a
    }

    pub fn yo(&mut self) -> Vec<bool> {
        vec![]
    }

    pub fn destroy(self) {}
}

trait MyTrait {
    fn trait_method(&mut self, x: i32) -> i32;
}

#[c_compatible]
impl MyTrait for MyStruct {
    fn trait_method(&mut self, x: i32) -> i32 {
        0 + x
    }
}

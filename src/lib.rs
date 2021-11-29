#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
include!("binding.rs");

mod test {
    #[test]
    fn new_device() {
        unsafe { crate::rtcNewDevice(std::ptr::null()); }
    }
}

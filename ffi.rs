use std::libc::{c_char, c_int, c_void};

pub static AO_ENODRIVER: int = 1;
pub static AO_ENOTFILE: int = 2;
pub static AO_ENOTLIVE: int = 3;
pub static AO_EBADOPTION: int = 4;
pub static AO_EOPENDEVICE: int = 5;
pub static AO_EOPENFILE: int = 6;
pub static AO_EFILEEXISTS: int = 7;
pub static AO_EBADFORMAT: int = 8;
pub static AO_EFAIL: int = 100;

#[link(name="ao")]
extern "C" {
    pub fn ao_initialize();
    pub fn ao_shutdown();

    pub fn ao_driver_id(short_name: *c_char) -> c_int;
    pub fn ao_default_driver_id() -> c_int;

    pub fn ao_driver_info(driver_id: c_int) -> *ao_info;
    
    pub fn ao_append_option(options: **ao_option, key: *c_char, value: *c_char) -> c_int;

    pub fn ao_open_live(driver_id: c_int,
                    format: *ao_sample_format, options: *ao_option) -> *ao_device;
    pub fn ao_open_file(driver_id: c_int, filename: *c_char, overwrite: c_int,
                    format: *ao_sample_format, options: *ao_option) -> *ao_device;

    pub fn ao_close(device: *ao_device) -> c_int;

    pub fn ao_play(device: *ao_device, output_samples: *c_char, num_bytes: u32) -> c_int;
}

pub struct ao_info {
    flavor: c_int,
    name: *c_char,
    short_name: *c_char,
    comment: *c_char,
    preferred_byte_format: c_int,
    priority: c_int,
    options: **c_char,
    option_count: c_int,
}

pub struct ao_option {
    key: *c_char,
    value: *c_char,
    next: *ao_option
}

// Opaque struct
pub type ao_device = c_void;

pub struct ao_sample_format {
    bits: c_int,
    rate: c_int,
    channels: c_int,
    byte_format: c_int,
    matrix: *c_char
}

pub static AO_FMT_LITTLE: int = 1;
pub static AO_FMT_BIG: int = 2;
pub static AO_FMT_NATIVE: int = 4;

use libc::{c_char, c_int, c_void};

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

    pub fn ao_driver_id(short_name: *const c_char) -> c_int;
    pub fn ao_default_driver_id() -> c_int;

    pub fn ao_driver_info(driver_id: c_int) -> *const ao_info;
    
    pub fn ao_append_option(options: *mut *mut ao_option,
                            key: *const c_char,
                            value: *const c_char) -> c_int;

    pub fn ao_open_live(driver_id: c_int,
                        format: *const ao_sample_format,
                        options: *const ao_option) -> *mut ao_device;
    pub fn ao_open_file(driver_id: c_int,
                        filename: *const c_char,
                        overwrite: c_int,
                        format: *const ao_sample_format,
                        options: *const ao_option) -> *mut ao_device;

    pub fn ao_close(device: *mut ao_device) -> c_int;

    pub fn ao_play(device: *mut ao_device,
                   output_samples: *const c_char,
                   num_bytes: u32) -> c_int;
}

pub struct ao_info {
    pub flavor: c_int,
    pub name: *const c_char,
    pub short_name: *const c_char,
    pub comment: *const c_char,
    pub preferred_byte_format: c_int,
    pub priority: c_int,
    pub options: *const *const c_char,
    pub option_count: c_int,
}

pub static AO_TYPE_LIVE: c_int = 1;
pub static AO_TYPE_FILE: c_int = 2;

pub struct ao_option {
    key: *mut c_char,
    value: *mut c_char,
    next: *mut ao_option
}

// Opaque struct
pub type ao_device = c_void;

pub struct ao_sample_format {
    pub bits: c_int,
    pub rate: c_int,
    pub channels: c_int,
    pub byte_format: c_int,
    pub matrix: *const c_char
}

pub static AO_FMT_LITTLE: int = 1;
pub static AO_FMT_BIG: int = 2;
pub static AO_FMT_NATIVE: int = 4;

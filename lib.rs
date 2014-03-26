#[crate_id = "ao#0.1"];
#[desc = "libao bindings"];
#[license = "BSD"];
#[crate_type = "dylib"];

#[allow(dead_code)];

use std::libc::c_int;
use std::os;
use std::ptr;

#[allow(non_camel_case_types)]
mod ffi;

pub type AoResult<T> = Result<T, AoError>;

#[deriving(Eq, Show)]
pub enum AoError {
    /// No driver is available. This means either:
    ///  * There is no driver matching the requested name
    ///  * There are no usable audio output devices
    NoDriver = ffi::AO_ENODRIVER,
    NotFile = ffi::AO_ENOTFILE,
    NotLive = ffi::AO_ENOTLIVE,
    BadOption = ffi::AO_EBADOPTION,
    OpenDevice = ffi::AO_EOPENDEVICE,
    FileExists = ffi::AO_EFILEEXISTS,
    BadFormat = ffi::AO_EBADFORMAT,
    Unknown = ffi::AO_EFAIL
}

impl AoError {
    fn from_errno() -> AoError {
        match os::errno() {
            ffi::AO_ENODRIVER => NoDriver,
            ffi::AO_ENOTFILE => NotFile,
            ffi::AO_ENOTLIVE => NotLive,
            ffi::AO_EBADOPTION => BadOption,
            ffi::AO_EOPENDEVICE => OpenDevice,
            ffi::AO_EFILEEXISTS => FileExists,
            ffi::AO_EBADFORMAT => BadFormat,
            _ => Unknown
        }
    }
}

/// Must be called before performing any other operations.
///
/// Initializes the libao internals, including loading plugins and reading
/// configuration files.
///
/// This function must be called only once before a corresponding `shutdown`,
/// and must be called from the main thread of an application.
pub fn initialize() {
    unsafe {
        ffi::ao_initialize()
    }
}

/// Shuts down libao
///
/// All open devices must be closed before calling this function.
pub fn shutdown() {
    unsafe {
        ffi::ao_shutdown()
    }
}

pub struct SampleFormat {
    /// Bits per sample
    sample_bits: uint,
    /// Samples per second (per channel)
    sample_rate: uint,
    /// Number of channels
    channels: uint,
    /// Byte order of samples. Ignored if `sample_bits = 8`
    byte_order: Endianness,
    /// Maps input channels to output locations in a comma-separated list.
    ///
    /// For example, "L,R" specifies channel 0 as left and 1 as right, or
    /// "L,R,C,LFE,BR,BL" for a 5.1 FLAC file.
    ///
    /// Refer to the [`matrix` documentation](https://www.xiph.org/ao/doc/ao_sample_format.html)
    /// for additional information and examples.
    matrix: Option<~str>
}

impl SampleFormat {
    fn with_native<T>(&self, f: |*ffi::ao_sample_format| -> T) -> T {
        // The caller of ao_open_* functions retains ownership of the ao_format
        // it passes in, but the native representation owns a raw C string.
        // We must ensure the raw C string is freed, so the actual
        // ao_sample_format never leaves this scope.
        let mut native = ffi::ao_sample_format {
            bits: self.sample_bits as c_int,
            rate: self.sample_rate as c_int,
            channels: self.channels as c_int,
            byte_format: self.byte_order as c_int,
            matrix: ptr::null()
        };

        match self.matrix {
            None => f(&native),
            Some(ref s) => s.with_c_str(|s| {
                native.matrix = s;
                f(&native)
            })
        }
    }
}

pub enum Endianness {
    /// Least-significant byte first
    Little = ffi::AO_FMT_LITTLE,
    /// Most-significant byte first
    Big = ffi::AO_FMT_BIG,
    /// Current machine's default byte order
    Native = ffi::AO_FMT_NATIVE
}

pub struct Driver(c_int);

impl Driver {
    /// Gets an output driver by name
    ///
    /// See the [libao docs](https://www.xiph.org/ao/doc/drivers.html) for
    /// the drivers provided by default. Note that this is not an exhaustive
    /// list, as user plugins may provide additional drivers.
    pub fn by_name(name: &str) -> Option<Driver> {
        Driver::wrap(name.with_c_str(|name| unsafe {
            ffi::ao_driver_id(name)
        }))
    }
    
    /// Gets the default live output driver
    ///
    /// A user-specified default driver will be chosen if configured, otherwise
    /// a driver that will work on the current system is selected.
    pub fn default() -> Option<Driver> {
        Driver::wrap(unsafe {
            ffi::ao_default_driver_id()
        })
    }

    /// Translates `ao_*_driver_id()` results
    fn wrap(id: c_int) -> Option<Driver> {
        if id == -1 {
            None
        } else {
            Some(Driver(id))
        }
    }
}

pub struct Device(*ffi::ao_device);

impl Device {
    /// Constructs a Device from `*ao_device`.
    fn result(handle: *ffi::ao_device) -> AoResult<Device> {
        unsafe {
            match handle.to_option() {
                None => {
                    Err(AoError::from_errno())
                }
                Some(d) => Ok(Device(d))
            }
        }
    }

    /// Opens a live audio output device.
    ///
    /// ## Errors
    ///
    ///  * `NotLive`: the specified driver is not a live output device
    ///  * `BadOption`: a specified valid option has an invalid value
    ///  * `OpenDevice`: cannot open the output device
    ///  * `Unknown`: Unspecified failure
    pub fn open(driver: &Driver, format: &SampleFormat/*,
            options: Option<OutputOptions>*/) -> AoResult<Device> {
        let &Driver(id) = driver;
        format.with_native(|f| {
            Device::result(
                unsafe {
                    ffi::ao_open_live(id, f, ptr::null())
                }
            )}
        )
    }

    /// Opens a file for audio output.
    ///
    /// `path` specifies the file to write to, and `overwrite` will
    /// automatically replace any existing file if `true`.
    ///
    /// ## Errors
    ///
    /// Returns similar errors to `open`, with several additions:
    /// `OpenFile` if the specified file cannot be opened, and
    /// `FileExists` if the file exists and `overwrite` is `false`.
    pub fn open_file(driver: Driver, format: &SampleFormat,
                     path: &Path, overwrite: bool/*,
                     options: Option<OutputOptions>*/) -> AoResult<Device> {
        let Driver(id) = driver;
        format.with_native(|f| {
            path.with_c_str(|filename| unsafe {
                Device::result(
                    ffi::ao_open_file(id, filename, overwrite as c_int,
                                      f, ptr::null())
                )
            })
        })
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let Device(handle) = *self;
        unsafe {
            ffi::ao_close(handle);
        }
    }
}

#![crate_id = "ao#0.1"]
#![desc = "libao bindings"]
#![license = "BSD"]
#![crate_type = "lib"]

#![feature(macro_rules,unsafe_destructor)]

extern crate libc;
extern crate rand;

use libc::c_int;
use std::intrinsics::size_of;
use std::os;
use std::ptr;
use std::rc::Rc;
use std::raw::Repr;

//pub mod pipeline;

#[allow(non_camel_case_types, dead_code)]
mod ffi;

pub type AoResult<T> = Result<T, AoError>;

#[deriving(PartialEq, Eq, Show)]
pub enum AoError {
    /// No driver is available. This means either:
    ///  * There is no driver matching the requested name
    ///  * There are no usable audio output devices
    NoDriver = ffi::AO_ENODRIVER,
    NotFile = ffi::AO_ENOTFILE,
    NotLive = ffi::AO_ENOTLIVE,
    BadOption = ffi::AO_EBADOPTION,
    OpenDevice = ffi::AO_EOPENDEVICE,
    OpenFile = ffi::AO_EOPENFILE,
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

/// Type bound for sample formats
///
/// All types that implement `Sample` should be raw enough to permit output
/// without additional processing. Conspicuously missing from the default impls
/// is a 24-bit type, simply because there isn't a Rust-native 24-bit type.
pub trait Sample: Num + NumCast {
    fn max() -> Self;
    fn min() -> Self;
}

macro_rules! sample_impl(
    ($t:ty, $min:expr .. $max:expr) => (
        impl Sample for $t {
            fn max() -> $t { $min }
            fn min() -> $t { $max }
        }
    );
    ($t:ty) => (
        sample_impl!($t, ::std::num::Bounded::min_value() .. ::std::num::Bounded::max_value())
    )
)

sample_impl!(i8)
sample_impl!(i16)
sample_impl!(i32)
sample_impl!(f32, -1.0 .. 1.0)
sample_impl!(f64, -1.0 .. 1.0)

/// Describes audio sample formats.
///
/// Used to specify the format which data will be fed to a Device
pub struct SampleFormat<S> {
    /// Samples per second (per channel)
    pub sample_rate: uint,
    /// Number of channels
    pub channels: uint,
    /// Byte order of samples.
    pub byte_order: Endianness,
    /// Maps input channels to output locations in a comma-separated list.
    ///
    /// For example, "L,R" specifies channel 0 as left and 1 as right, or
    /// "L,R,C,LFE,BR,BL" for a 5.1 FLAC file.
    ///
    /// Refer to the [`matrix` documentation](https://www.xiph.org/ao/doc/ao_sample_format.html)
    /// for additional information and examples.
    pub matrix: Option<String>
}

impl<S: Int> SampleFormat<S> {
    fn with_native<T>(&self, f: |*ffi::ao_sample_format| -> T) -> T {
        let sample_size = unsafe {
            size_of::<S>() * 8
        };
        // The caller of ao_open_* functions retains ownership of the ao_format
        // it passes in, but the native representation owns a raw C string.
        // We must ensure the raw C string is freed, so the actual
        // ao_sample_format never leaves this scope.
        let mut native = ffi::ao_sample_format {
            bits: sample_size as c_int,
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

/// Machine byte order.
pub enum Endianness {
    /// Least-significant byte first
    Little = ffi::AO_FMT_LITTLE,
    /// Most-significant byte first
    Big = ffi::AO_FMT_BIG,
    /// Machine's default byte order
    Native = ffi::AO_FMT_NATIVE
}

/// The master of all things libao.
pub struct AO;

impl AO {
    /// Initializes libao internals, including loading plugins and reading
    /// configuration files.
    ///
    /// This function must be called only once before a corresponding `shutdown`,
    /// and must be called from the main thread of an application.
    pub fn init() -> Rc<AO> {
        unsafe {
            ffi::ao_initialize();
        };
        Rc::new(AO)
    }

    /// Gets the specified output driver or default.
    ///
    /// `name` specifies the name of the output driver to use, or pass the null
    /// string (`""`) to get the default driver.
    ///
    /// Returns `None` if the driver is not available.
    ///
    /// # Drivers
    ///
    /// See the [libao docs](https://www.xiph.org/ao/doc/drivers.html) for
    /// the drivers provided by default. Note that this is not an exhaustive
    /// list, as user plugins may provide additional drivers.
    ///
    /// The default driver may be user-specified, or it will be automatically
    /// chosen to be a live output supported by the current platform. This
    /// implies that the default driver will not necessarily be a live output.
    pub fn get_driver(&self, name: &str) -> Option<Driver> {
        let id = if name != "" {
            name.with_c_str(|name| unsafe {
                ffi::ao_driver_id(name)
            })
        } else {
            unsafe {
                ffi::ao_default_driver_id()
            }
        };

        if id == -1 {
            None
        } else {
            Some(DriverID(id))
        }
    }


}

#[unsafe_destructor]
impl Drop for AO {
    fn drop(&mut self) {
        unsafe {
            ffi::ao_shutdown();
        }
    }
}

pub enum Driver {
    DriverID(c_int),
    DriverName(String)
}

pub enum DriverType {
    Live,
    File
}

#[deriving(Show)]
pub struct DriverInfo {
    //flavor: DriverType,
    name: String,
    //short_name: CString,
    //comment: CString,
}

impl Driver {
    /*
    pub fn get_info(&self) -> Option<DriverInfo> {
        let &Driver(id) = self;
        unsafe {
            ffi::ao_driver_info(id).to_option().map(|info| {
                let name = CString::new(info.name, false).as_str().unwrap().into_owned();
                DriverInfo {
                    name: name
//                    short_name: CString::new(info.short_name, false),
//                    comment: CString::new(info.comment, false)
                }
            })
        }
    }*/

    /// Get the raw (libao internal) driver ID corresponding to this Driver.
    fn as_raw(&self, lib: &AO) -> AoResult<c_int> {
        match *self {
            DriverID(id) => Ok(id),
            DriverName(ref s) => match lib.get_driver(s.as_slice()) {
                None => Err(NoDriver),
                Some(DriverID(id)) => Ok(id),
                Some(DriverName(_)) => unreachable!()
            }
        }
    }
}

pub struct Device<'a, S> {
    id: *ffi::ao_device,
    //lib_instance: &'a AO
}

impl<'a, S: Int> Device<'a, S> {
    pub fn live(lib: &'a AO, driver: Driver, format: &SampleFormat<S>/*,
                       options: */) -> AoResult<Device<'a, S>> {
        let id = try!(driver.as_raw(lib));

        let handle = format.with_native(|f| unsafe {
            ffi::ao_open_live(id, f, ptr::null())
        });

        Device::<S>::init(lib, handle)
    }

    /// Opens a file for audio output.
    ///
    /// `path` specifies the file to write to, and `overwrite` will
    /// automatically replace any existing file if `true`.
    ///
    /// # Errors
    ///
    /// Returns similar errors to `open`, with several additions:
    ///
    ///  * `OpenFile`: the specified file cannot be opened, and
    ///  * `FileExists`: the file exists and `overwrite` is `false`.
    pub fn file(lib: &'a AO, driver: Driver, format: &SampleFormat<S>,
                     path: &Path, overwrite: bool/*,
                     options: */) -> AoResult<Device<'a, S>> {
        let id = try!(driver.as_raw(lib));
        let handle = format.with_native(|f| {
            path.with_c_str(|filename| unsafe {
                ffi::ao_open_file(id, filename, overwrite as c_int, f, ptr::null())
            })
        });

        Device::<S>::init(lib, handle)
    }

    /// Inner helper to finish Device init given a FFI handle.
    #[allow(unused_variable)]
    fn init(lib: &'a AO, handle: *ffi::ao_device) -> AoResult<Device<'a, S>> {
        let opt = unsafe {
            handle.to_option()
        };

        match opt {
            None => {
                Err(AoError::from_errno())
            }
            Some(d) => Ok(Device {
                id: d,
                //lib_instance: lib
            })
        }
    }

    pub fn play(&self, samples: &[S]) {
        let slice = samples.repr();
        unsafe {
            let len = slice.len * size_of::<S>();
            ffi::ao_play(self.id, slice.data as *i8, len as u32);
        }
    }
}

#[unsafe_destructor]
impl<'a, S> Drop for Device<'a, S> {
    fn drop(&mut self) {
        unsafe {
            ffi::ao_close(self.id);
        }
    }
}

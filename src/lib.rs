#![crate_name = "ao"]
#![doc(html_root_url = "http://www.rust-ci.org/tari/rust-ao/doc/ao/")]
#![desc = "libao bindings"]
#![license = "BSD"]
#![crate_type = "lib"]

#![deny(dead_code, missing_doc)]
#![feature(macro_rules,unsafe_destructor)]

//! Bindings to libao, a low-level library for audio output.
//!
//! ```no_run
//! use ao::{AO, SampleFormat, Native};
//! use std::num::FloatMath;
//!
//! fn main() {
//!     let lib = AO::init();
//!     let format = SampleFormat::<i16, &'static str>::new(44100, 1, Native, None);
//!     let driver = match lib.get_driver("wav") {
//!         Some(d) => d,
//!         None => panic!("No such driver: \"wav\"")
//!     };
//!     
//!     match driver.open_file(&format, &Path::new("out.wav"), false) {
//!         Ok(d) => {
//!             let samples = Vec::<i16>::from_fn(44100, |i| {
//!                 ((1.0 / 44100.0 / 440.0 * i as f32).sin() * 32767.0) as i16
//!             });
//!             d.play(samples.as_slice());
//!         }
//!         Err(e) => {
//!             println!("Failed to open output file: {}", e);
//!         }
//!     }
//! }
//! ```

extern crate libc;

use libc::c_int;
use std::c_str::CString;
use std::fmt;
use std::intrinsics::size_of;
use std::kinds::marker::{ContravariantLifetime, InvariantType};
use std::os;
use std::sync::atomic::{AtomicBool, Release, AcqRel, INIT_ATOMIC_BOOL};
use std::ptr;

#[allow(non_camel_case_types, dead_code)]
mod ffi;
pub mod auto;

/// Output for libao functions that may fail.
pub type AoResult<T> = Result<T, AoError>;

#[deriving(PartialEq, Eq, Show)]
/// Result of (most) operations that may fail.
pub enum AoError {
    /// No driver is available.
    ///
    /// This means either:
    ///  * There is no driver matching the requested name
    ///  * There are no usable audio output devices
    NoDriver = ffi::AO_ENODRIVER as int,
    /// The specified driver does not do file output.
    NotFile = ffi::AO_ENOTFILE as int,
    /// The specified driver does not do live output.
    NotLive = ffi::AO_ENOTLIVE as int,
    /// A known driver option has an invalid value.
    BadOption = ffi::AO_EBADOPTION as int,
    /// Could not open the output device.
    ///
    /// For example, if `/dev/dsp` could not be opened with the OSS driver.
    OpenDevice = ffi::AO_EOPENDEVICE as int,
    /// Could not open the output file.
    OpenFile = ffi::AO_EOPENFILE as int,
    /// The specified file already exists.
    FileExists = ffi::AO_EFILEEXISTS as int,
    /// The requested stream format is not supported.
    ///
    /// This is usually the result of an invalid channel matrix.
    BadFormat = ffi::AO_EBADFORMAT as int,
    /// Unspecified error.
    Unknown = ffi::AO_EFAIL as int,
}

impl AoError {
    fn from_errno() -> AoError {
        match os::errno() as c_int {
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
pub trait Sample : Copy {
    /// Number of channels each value of this type contains.
    fn channels(&self) -> uint;
}

macro_rules! sample_impl(
    ($t:ty) => (
        impl Sample for $t {
            fn channels(&self) -> uint { 1 }
        }
    );
    (channels $w:expr) => (
        impl<S: Sample> Sample for [S, ..$w] {
            fn channels(&self) -> uint { $w }
        }
    )
)
sample_impl!(i8)
sample_impl!(i16)
sample_impl!(i32)
sample_impl!(channels 2)

/// Describes audio sample formats.
///
/// Used to specify the format with which data will be fed to a Device.
pub struct SampleFormat<T, S> {
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
    pub matrix: Option<S>,
    marker: InvariantType<T>,
}

impl<T: Sample, S: Str> SampleFormat<T, S> {
    /// Construct a sample format specification.
    pub fn new(sample_rate: uint, channels: uint, byte_order: Endianness,
               matrix: Option<S>) -> SampleFormat<T, S> {
        SampleFormat {
            sample_rate: sample_rate,
            channels: channels,
            byte_order: byte_order,
            matrix: matrix,
            marker: InvariantType
        }
    }

    fn with_native<U>(&self, f: |*const ffi::ao_sample_format| -> U) -> U {
        let sample_size = unsafe {
            size_of::<T>() * 8
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
            Some(ref s) => s.as_slice().with_c_str(|s| {
                native.matrix = s;
                f(&native)
            })
        }
    }
}

/// Sample byte ordering.
#[deriving(PartialEq, Eq)]
pub enum Endianness {
    /// Least-significant byte first
    Little = ffi::AO_FMT_LITTLE as int,
    /// Most-significant byte first
    Big = ffi::AO_FMT_BIG as int,
    /// Machine's default byte order
    Native = ffi::AO_FMT_NATIVE as int,
}

/// Library owner.
///
/// Initialization of this object loads plugins and system/user configuration
/// files. There must be only one instance of this object live at a given time.
///
/// Behind the scenes, this object controls initialization of libao. It should
/// be created only from the main thread of your application, due to bugs in
/// some output drivers that can cause segfaults on thread exit.
pub struct AO;

static mut FFI_INITIALIZED: AtomicBool = INIT_ATOMIC_BOOL;

impl AO {
    /// Get the `AO`
    pub fn init() -> AO {
        unsafe {
            if FFI_INITIALIZED.compare_and_swap(false, true, AcqRel) {
                panic!("Attempted multiple instantiation of ao::AO")
            }
            ffi::ao_initialize();
        };
        AO
    }

    /// Gets the specified output driver or default.
    ///
    /// `name` specifies the name of the output driver to use, or pass the null
    /// string (`""`) to get the default driver.
    ///
    /// Returns `None` if the driver is not available.
    ///
    /// The default driver may be specified by the user or system
    /// configuration, otherwise it will be automatically chosen to be a live
    /// output supported by the current platform. This implies that the default
    /// driver will not necessarily be a live output.
    pub fn get_driver<'a>(&'a self, name: &str) -> Option<Driver<'a>> {
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
            Some(Driver {
                id: id,
                marker: ContravariantLifetime
            })
        }
    }
}

#[unsafe_destructor]
impl Drop for AO {
    fn drop(&mut self) {
        unsafe {
            ffi::ao_shutdown();
            FFI_INITIALIZED.store(false, Release);
        }
    }
}

/// The output type of a driver.
#[deriving(Show)]
pub enum DriverType {
    /// Live playback, such as a local sound card.
    Live,
    /// File output, such as to a `wav` file on disk.
    File
}

impl DriverType {
    fn from_c_int(n: c_int) -> DriverType {
        match n {
            ffi::AO_TYPE_FILE => File,
            ffi::AO_TYPE_LIVE => Live,
            n => panic!("Invalid AO_TYPE_*: {}", n)
        }
    }
}

/// Properties and metadata for a driver.
pub struct DriverInfo {
    /// Type of the driver (live or file).
    pub flavor: DriverType,
    /// Full name of driver.
    /// 
    /// May contain any single line of text.
    pub name: CString,
    /// Short name of driver.
    /// 
    /// This is the driver name used to refer to the driver when performing
    /// lookups. It contains only alphanumeric characters, and no whitespace.
    pub short_name: CString,
    /// A driver-specified comment.
    pub comment: Option<CString>,
}

impl fmt::Show for DriverInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{} \"{}\", {}>",
               self.name.as_str(),
               self.short_name.as_str(),
               self.flavor)
    }
}

/// An output driver.
///
/// This is an opaque handle.
pub struct Driver<'a> {
    id: c_int,
    marker: ContravariantLifetime<'a>,
}

impl<'a> Driver<'a> {
    /// Get the `DriverInfo` corresponding to this `Driver`.
    pub fn get_info(&self) -> Option<DriverInfo> {
        let id = self.id;

        unsafe {
            ffi::ao_driver_info(id).as_ref().map(|info| {
                DriverInfo {
                    name: CString::new(info.name, false),
                    short_name: CString::new(info.short_name, false),
                    comment: if info.comment.is_null() {
                        None
                    } else {
                        Some(CString::new(info.comment, false))
                    },
                    flavor: DriverType::from_c_int(info.flavor),
                }
            })
        }
    }

    /// Open a live output device.
    ///
    /// Returns `NotLive` if the specified driver is not a live output driver.
    /// In this case, open the device as a file output instead.
    pub fn open_live<T: Sample, S: Str>(&self,
            format: &SampleFormat<T, S>) -> AoResult<Device<'a, T>> {
        let handle = format.with_native(|f| unsafe {
            ffi::ao_open_live(self.id, f, ptr::null())
        });

        Device::<'a, T>::init(handle)
    }

    /// Open a file output device.
    ///
    /// `path` specifies the file to write to, and `overwrite` will
    /// automatically replace any existing file if `true`.
    ///
    /// Returns `NotFile` if the requested driver is not a file output driver.
    pub fn open_file<T: Sample, S: Str>(&self,
            format: &SampleFormat<T, S>, file: &Path,
            overwrite: bool) -> AoResult<Device<'a, T>> {
        let handle = format.with_native(|f| {
            file.with_c_str(|filename| unsafe {
                ffi::ao_open_file(self.id, filename, overwrite as c_int, f, ptr::null())
            })
        });

        Device::<'a, T>::init(handle)
    }
}

/// An output device.
pub struct Device<'a, S> {
    id: *mut ffi::ao_device,
    marker0: ContravariantLifetime<'a>,
    marker1: InvariantType<S>,
}

impl<'a, S: Sample> Device<'a, S> {

    /// Inner helper to finish Device init given a FFI handle.
    fn init(handle: *mut ffi::ao_device) -> AoResult<Device<'a, S>> {
        if handle.is_null() {
            Err(AoError::from_errno())
        } else {
            Ok(Device {
                id: handle,
                marker0: ContravariantLifetime,
                marker1: InvariantType,
            })
        }
    }

    /// Plays packed samples through a device.
    ///
    /// For multi-channel output, channels are interleaved, such that positions
    /// in the `samples` slice for four ouput channels would be as so:
    ///
    /// ```ignore
    /// [c1, c2, c3, c4,    <-- time 1
    ///  c1, c2, c3, c4]    <-- time 2
    /// ```
    /// 
    /// In most cases this layout can be achieved as either an array
    /// or tuple. Again with 4 channels:
    ///
    /// ```ignore
    /// my_device.play(&[[0, 0, 0, 0], [0, 0, 0, 0]]);
    /// ```
    pub fn play(&self, samples: &[S]) {
        unsafe {
            let len = samples.len() * size_of::<S>();
            ffi::ao_play(self.id, samples.as_ptr() as *const i8, len as u32);
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

// Driver<'a> must not be able to outlive the &'a AO that created it.
// Unfortunately there's no #[compile_fail] for #[test] like
// #[should_fail].
/*
#[test]
fn test_driver_lifetime() {
    let driver: Driver;
    {
        let lib = AO::init();
        driver = lib.get_driver("").unwrap();
    }
    driver.get_info();
}
*/

// Device<S> must not accept samples of any type other than S.
/*
#[test]
fn test_sample_variance() {
    let lib = AO::init();
    let device = lib.get_driver("").unwrap().open_live::<i16>(&SampleFormat {
        sample_rate: 44100,
        channels: 1,
        byte_order: Native,
        matrix: None
    }).unwrap();
    // Invalid: does not match declared sample format
    device.play(&[0i32]);
    // OK
    device.play(&[0i16]);
}
*/

/// Task fails on multiple initialization.
#[test]
#[should_fail]
#[allow(unused_variable)]
fn test_multiple_instantiation() {
    let lib = AO::init();
    let lib2 = AO::init();
}

//! Automatic device format adjustment.
//!
//! Given a way to poll the properties of an incoming buffer of samples, this module provides a way
//! to automatically adjust the output `SampleFormat` so all data being fed to the output need not
//! have the same format. This is particularly useful for situations where non-homogenous inputs
//! can be switched to the same output, without requiring resampling prior to output.
//!
//! ```
//! use ao::AO;
//! use ao::auto::{SampleBuffer, AutoFormatDevice};
//!
//! struct Stereo(u16, u16);
//! 
//! impl SampleBuffer for [Stereo] {
//!     fn channels(&self) -> uint { 2 }
//!     fn sample_rate(&self) -> uint { 44100 }
//!     fn endianness(&self) -> ao::Endianness { ao::Native }
//!     fn sample_width(&self) -> uint { 16 }
//!     fn data<'a>(&self) -> &'a [u8] { 
//!         unsafe {
//!             ::std::mem::transmute(::std::raw::Slice {
//!                 data: self.as_ptr() as *const u8,
//!                 len: self.len() * 4
//!             })
//!         }
//!     }
//! }
//!
//! fn main() {
//!     let lib = AO::init();
//!     let driver = lib.get_driver("").expect("No default driver available");
//!     let mut device = AutoFormatDevice::new(driver, vec!["", "L", "L,R"]);
//!
//!     let data = vec![Stereo(16383, -16383)];
//!     device.play(data.as_slice()).unwrap();
//! }
//! ```

use super::{AoResult, Device, Driver, Sample, SampleFormat};
use super::{Endianness, Native};
use std::kinds::marker::InvariantType;
use std::mem;

/// A buffer containing samples.
///
/// Such buffer always has a defined number of channels and sample rate, in addition to the
/// parameters normally provided in a `SampleFormat` specification.
pub trait SampleBuffer for Sized? {
    /// Number of channels in this buffer.
    fn channels(&self) -> uint;
    /// Sample rate of this buffer, in Hz.
    fn sample_rate(&self) -> uint;
    /// Endianness of samples in this buffer.
    fn endianness(&self) -> Endianness;
    /// Bit width of samples in this buffer.
    fn sample_width(&self) -> uint;
    /// Provides access to the sample data.
    ///
    /// No processing is performed on this data; it is passed straight through to the underlying
    /// library.
    fn data<'a>(&self) -> &'a [u8];
}

enum DeviceFormat<'a> {
    Integer8(Device<'a, i8>),
    Integer16(Device<'a, i16>),
    Integer32(Device<'a, i32>),
}

impl<'a> DeviceFormat<'a> {
    fn sample_width(&self) -> uint {
        match *self {
            Integer8(_) => 8,
            Integer16(_) => 16,
            Integer32(_) => 32,
        }
    }

    fn new(driver: &Driver<'a>, width: uint,
           rate: uint, channels: uint, endianness: Endianness,
           matrix: Option<&str>) -> AoResult<DeviceFormat<'a>> {

        fn build_format<S: Sample>(rate: uint, channels: uint, order: Endianness,
                                   matrix: Option<&str>) -> SampleFormat<S, &str> {
            SampleFormat {
                sample_rate: rate,
                channels: channels,
                byte_order: order,
                matrix: matrix,
                marker: InvariantType
            }
        }

        match width {
            8 => {
                let format = build_format::<i8>(rate, channels, endianness, matrix);
                driver.open_live(&format).map(|x| Integer8(x))
            },
            16 => {
                let format = build_format::<i16>(rate, channels, endianness, matrix);
                driver.open_live(&format).map(|x| Integer16(x))
            },
            32 => {
                let format = build_format::<i32>(rate, channels, endianness, matrix);
                driver.open_live(&format).map(|x| Integer32(x))
            },
            x => panic!("AutoFormatDevice does not support {}-bit samples", x)
        }
    }
}

/// Automatically adjusts the output format according to incoming buffers.
///
/// This device adapter can automatically manage the underlying `Device` to ensure it always has
/// the correct sample format, so the format of incoming samples may change at runtime.
pub struct AutoFormatDevice<'a, S> {
    channels: uint,
    sample_rate: uint,
    endianness: Endianness,
    device: Option<DeviceFormat<'a>>,
    driver: Driver<'a>,
    matrixes: Vec<S>
}

impl<'a, S: Str> AutoFormatDevice<'a, S> {
    /// Construct a new AutoFormatDevice.
    ///
    /// Will be backed by the specified driver, and the `matrixes` is a list where an element's
    /// index maps to the number of channels. See `Sampleformat.matrix` for the format of each
    /// channel matrix.
    pub fn new(driver: Driver<'a>, matrixes: Vec<S>) -> AutoFormatDevice<'a, S> {
        AutoFormatDevice {
            channels: 0,
            sample_rate: 0,
            endianness: Native,
            device: None,
            driver: driver,
            matrixes: matrixes
        }
    }

    /// Play samples from a dynamic format buffer.
    /// 
    /// The underling device may be reopened, and returns `Err` if
    /// the format of the buffer is not supported.
    pub fn play(&mut self, data: &SampleBuffer) -> AoResult<()> {
        let channels = data.channels();
        let sample_rate = data.sample_rate();
        let sample_width = data.sample_width();
        let endianness = data.endianness();

        let must_reopen = match self.device {
            None => {
                true
            }
            Some(ref d) => {
                // Might need to reopen the device
                if channels != self.channels ||
                   sample_rate != self.sample_rate ||
                   endianness != self.endianness ||
                   sample_width != d.sample_width() {
                    true
               } else {
                   false
                }
            }
        };
        if must_reopen {
            self.device = Some(try!(
                self.open_device(sample_width, sample_rate, channels, endianness)
            ));
        }

        // If we didn't early return, our parameters are consistent with the sample buffer.
        self.channels = channels;
        self.sample_rate = sample_rate;
        self.endianness = endianness;

        // Do the playback
        let buffer = data.data();
        match self.device {
            Some(ref f) => {
                unsafe {
                    match *f {
                        Integer8(ref d) => d.play(mem::transmute(buffer)),
                        Integer16(ref d) => d.play(mem::transmute(buffer)),
                        Integer32(ref d) => d.play(mem::transmute(buffer)),
                    }
                }
            },
            None => unreachable!()
        }
        Ok(())
    }

    fn open_device(&self, width: uint, rate: uint, channels: uint,
                   endianness: Endianness) -> AoResult<DeviceFormat<'a>> {
        DeviceFormat::new(&self.driver, width, rate, channels, endianness,
                          self.matrix_for(channels))
    }

    fn matrix_for(&self, nchannels: uint) -> Option<&str> {
        if self.matrixes.len() <= nchannels {
            None
        } else {
            Some(self.matrixes[nchannels].as_slice())
        }
    }
}


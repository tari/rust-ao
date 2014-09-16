use super::{AoResult, Device, Driver, Sample, SampleFormat};
use super::{Endianness, Native};
use std::kinds::marker::InvariantType;
use std::mem;

pub trait SampleBuffer {
    fn channels(&self) -> uint;
    fn sample_rate(&self) -> uint;
    fn endianness(&self) -> Endianness;
    fn sample_width(&self) -> uint;
    fn data(&self) -> (uint, *const u8);
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
            x => fail!("AutoFormatDevice does not support {}-bit samples", x)
        }
    }
}

pub struct AutoFormatDevice<'a, S> {
    channels: uint,
    sample_rate: uint,
    endianness: Endianness,
    device: Option<DeviceFormat<'a>>,
    driver: Driver<'a>,
    matrixes: Vec<S>
}

impl<'a, S: Str> AutoFormatDevice<'a, S> {
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
        let (nsamples, buffer) = data.data();
        unsafe {
            let buffer = ::std::raw::Slice {
                data: buffer,
                len: nsamples
            };
            match self.device {
                Some(ref f) => {
                    match *f {
                        Integer8(ref d) => d.play(mem::transmute(buffer)),
                        Integer16(ref d) => d.play(mem::transmute(buffer)),
                        Integer32(ref d) => d.play(mem::transmute(buffer)),
                    }
                },
                None => unreachable!()
            }
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


#![crate_id = "ao_demo#0.1"]
#![desc = "libao bindings demo"]
#![license = "BSD"]
#![crate_type = "bin"]

extern crate ao;
extern crate rand;

use ao::AO;
use rand::Rng;
use rand::distributions::IndependentSample;
use rand::distributions::normal::Normal;
use std::num::{Bounded, cast};

fn main() {
    let args = ::std::os::args();
    let driver_name = if args.len() > 1 {
        args[1]
    } else {
        ~""
    };

    println!("Using driver {}", driver_name);
    let format: ao::SampleFormat<i16> = ao::SampleFormat {
        sample_rate: 44100,
        channels: 1,
        byte_order: ao::Native,
        matrix: None
    };

    let path = Path::new("out.wav");
    let device = match ao::Device::file(AO::init(),
                                        ao::DriverName(driver_name),
                                        &format,
                                        &path, true) {
        Ok(d) => d,
        Err(e) => fail!("Failed to open device: {}", e)
    };

    // Generate some CD-quality quite noise with range to 5 standard deviations
    let limit: i16 = Bounded::max_value();
    let mut rng = rand::StdRng::new();
    let noise = WhiteNoise::new(&mut rng, limit / 5);

    let samples: Vec<i16> = noise.take(44100).collect();
    device.play(samples.as_slice());

}

/// Guassian white noise generator
struct WhiteNoise<'a, R, T> {
    rng: &'a mut R,
    normal: Normal
}

impl<'a, R, T: Primitive> WhiteNoise<'a, R, T> {
    fn new(rng: &'a mut R, std_dev: T) -> WhiteNoise<'a, R, T> {
        WhiteNoise {
            rng: rng,
            normal: Normal::new(0f64, cast(std_dev).unwrap())
        }
    }
}

impl<'a, R: Rng, T: Primitive> Iterator<T> for WhiteNoise<'a, R, T> {
    fn next(&mut self) -> Option<T> {
        let sample = self.normal.ind_sample(self.rng);

        // Clamp sample to range of T to ensure cast will succeed
        let min = cast::<T, f64>(Bounded::min_value()).unwrap();
        let max = cast::<T, f64>(Bounded::max_value()).unwrap();

        let out = if sample < min {
            min
        } else if sample > max {
            max
        } else {
            sample
        };
        
        ::std::num::cast::<f64, T>(out)
    }
}

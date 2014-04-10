#![crate_id = "ao_demo#0.1"]
#![desc = "libao bindings demo"]
#![license = "BSD"]
#![crate_type = "bin"]

extern crate ao;
extern crate rand;

use ao::AO;
use ao::pipeline::{Source, Convert, WhiteNoise};

struct DeviceSink<F, S> {
    src: S,
    dev: ao::Device<F>
}

// libao doesn't do float output; just integer samples
impl<F: Int, S: Source<F>> DeviceSink<F, S> {
    fn new(dev: ao::Device<F>, src: S) -> DeviceSink<F, S> {
        DeviceSink {
            src: src,
            dev: dev
        }
    }

    fn run(&mut self, samples: uint) -> uint {
        let mut done: uint = 0;

        while done < samples {
            match self.src.next() {
                None => return done,
                Some(n) => {
                    let data = if n.len() > (samples - done) {
                        n.slice_to(samples - done)
                    } else {
                        n.as_slice()
                    };
                    done += data.len();
                    self.dev.play(data);
                }
            }
        }
        done
    }
}


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
    let mut rng = match rand::StdRng::new() {
        Ok(r) => r,
        Err(e) => fail!("StdRng::new(): {}", e.desc)
    };
    let mut pipeline = DeviceSink::new(device,
        Convert::<f64, i16>::new(
            ~WhiteNoise::new(&mut rng, 1.0) as ~Source<f64>
        )
    );
    pipeline.run(44100);
}


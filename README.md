# Rust-AO
libao[libao] bindings for Rust.

# Usage

Build with `cargo`:

    cargo build

Build documentation with `rustdoc`, rooted at `doc/ao/index.html`:

    rustdoc src/lib.rs

Write programs:

    extern crate ao;

    use ao::{AO, SampleFormat, Native, Device, DriverName};
    use std::num::FloatMath;

    fn main() {
        let lib = AO::init();
        let format: SampleFormat<i16> = SampleFormat {
            sample_rate: 44100,
            channels: 1,
            byte_order: Native,
            matrix: None
        };
        let device = Device::file(&lib, DriverName("wav"), &format,
                                  &Path::new("out.wav"), false);
        match device {
            Ok(d) => {
                let samples = Vec::<i16>::from_fn(44100, |i| {
                    ((1.0 / 44100.0 / 440.0 * i as f32).sin() * 32767.0) as i16
                });
                d.play(samples.as_slice());
            }
            Err(e) => {
                println!("Failed to open output file: {}", e);
            }
        }
    }


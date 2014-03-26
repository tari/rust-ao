#[crate_id = "ao_demo#0.1"];
#[desc = "libao bindings demo"];
#[license = "BSD"];
#[crate_type = "bin"];

extern crate ao;

fn main() {
    ao::initialize();

    {
        let driver = match ao::Driver::default() {
            None => fail!("Couldn't get a default driver"),
            Some(d) => d
        };

        let format = ao::SampleFormat {
            sample_bits: 16,
            sample_rate: 44100,
            channels: 2,
            byte_order: ao::Native,
            matrix: None
        };

        let device = match ao::Device::open(&driver, &format) {
            Ok(d) => d,
            Err(e) => fail!("Failed to open device: {}", e)
        };
    }

    ao::shutdown();
}

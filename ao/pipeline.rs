use super::Sample;
use rand::Rng;
use rand::distributions::IndependentSample;
use rand::distributions::normal::Normal;

// TODO Can avoid allocations by yielding &[T] rather than ~[T]. We can still
// permit parallelism by having a Buffer element that copies.
pub trait Source<T>: Iterator<~[T]> { }

/*
trait Sink<A, S> {
    fn attach<'a>(&'a mut self, prev: S) -> &'a mut Self;
}
*/

/*
// Source of &[T] for pipelines.
struct BufferSource<T>;

impl<T> for BufferSource<T> {
    fn new() -> BufferSource<T> {

    }
}

impl<T> Iterator<&[T]> for BufferSource<T> {

}
*/

/// Converts sample formats.
pub struct Convert<A, B> {
    // While I'd rather avoid trait objects, it's necessary here simply for
    // readability's sake. With a type bound S: Source<A>, type inference has
    // trouble and the user basically needs to encode the entire pipeline up
    // to this point in S.
    src: ~Source<A>
}

// These implementations are explicit because it's easier than mucking with
// numeric traits, and the actual conversion depends on which type is wider.
//
// Sample format conversions should be as fast as possible, so it's also easier
// to ensure speed with explicit implementations.

impl<A: Sample, B: Sample> Convert<A, B> {
    pub fn new(src: ~Source<A>) -> Convert<A, B> {
        Convert {
            src: src
        }
    }
}

impl Source<i16> for Convert<f64, i16> { }

impl Iterator<~[i16]> for Convert<f64, i16> {
    fn next(&mut self) -> Option<~[i16]> {
        let out: ~[i16] = self.src.next().unwrap().iter().map(|&s| {
            0
        }).collect();
        Some(out)
    }
}

/// Guassian white noise generator
pub struct WhiteNoise<'a, R> {
    rng: &'a mut R,
    normal: Normal,
    block_size: uint
}

impl<'a, R> WhiteNoise<'a, R> {
    pub fn new(rng: &'a mut R, amplitude: f64) -> WhiteNoise<'a, R> {
        WhiteNoise {
            // 4k buffer; a single VM page on most systems
            block_size: 512,
            rng: rng,
            // TODO there's probably a way to compute the standard deviation
            // we want for an average amplitude.
            normal: Normal::new(0f64, amplitude / 3.0)
        }
    }
}

impl<'a, R: Rng> Source<f64> for WhiteNoise<'a, R> { }

impl<'a, R: Rng> Iterator<~[f64]> for WhiteNoise<'a, R> {
    fn next(&mut self) -> Option<~[f64]> {
        Some(Vec::from_fn(self.block_size, |_| {
            let x = self.normal.ind_sample(self.rng);
            if x > 1.0 {
                1.0
            } else if x < -1.0 {
                -1.0
            } else {
                x
            }
        }).move_iter().collect())
    }
}

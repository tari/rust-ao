# Rust-AO
[libao] bindings for Rust.

[libao]: https://www.xiph.org/ao/

# Usage

Build with `cargo`:

    cargo build

Build documentation with `rustdoc`, rooted at `doc/ao/index.html`:

    cargo doc

Run tests. Tests must not be run in parallel because libao may only be
instantiated once in a given _process_. Running tests concurrently
can cause race conditions on library initialization, causing spurious
test failure:

    REST_TEST_TASKS=1 cargo test

Examples are included in the documentation.

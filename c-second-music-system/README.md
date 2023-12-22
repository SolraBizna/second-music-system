This is a nearly-complete C binding for the Second Music System. It follows the Rust API as closely as possible. It is thoroughly documented in the header file.

# Installation

Install Rust. [The easiest way is to use rustup.](https://www.rust-lang.org/learn/get-started)

If you have a UNIX-like environment, install a Nightly toolchain (`rustup install nightly`) and:

```sh
cd ...path/to/c-second-music-system
./setup.sh --nightly --build
sudo ./setup.sh --nightly --install
```

(If you don't mind the binary being more than twice as large, you can do without Nightly and omit `--nightly` from the above commands.)

`setup.sh` has several options that control it. Try `./setup.sh --help` for usage information.

In other environments, `cargo build --release` will give you a static library in `../target/release`. Install that in the appropriate place for your environment, likewise the headers from `include`.

# Legalese

Second Music System is copyright 2022 and 2023 Solra Bizna and Noah Obert. It is licensed under either of:

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or
   <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the Second Music System crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

# Compat shim to serialize/deserialize values from the ark ecosystem with serde

The [arkworks-rs](https://arkworks.rs/) ecosystem provides serialization features via its [ark-serialize](https://crates.io/crates/ark-serialize) crate. This crate adds shims to serialize and deserialize all types of `arkworks-rs` that implement the traits from `ark-serialize` with [serde](https://serde.rs).

## License

This crate is licensed under [Apache-2.0](LICENSE-APACHE) or the [MIT](LICENSE-MIT) license.

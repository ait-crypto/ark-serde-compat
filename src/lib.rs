#![doc = include_str!("../README.md")]
//! ## Usage
//!
//! This crate is intended to be used with serde's `#[serde(serialize_with = "ark_serde_compat")]`, `#[serde(deserialize_with = "ark_serde_compat")]`, and `#[serde(with = "ark_serde_compat")]`. Please see serde's documentation on how to use these attributes.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
#![warn(clippy::use_self)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::{any::type_name, fmt, marker::PhantomData};

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use serde::{
    Deserializer, Serializer,
    de::{self, Visitor},
    ser::Error as _,
};

/// Serialize and deserialize to and from compressed representations.
pub mod compressed {
    use super::*;

    struct ByteVisitor<T>(PhantomData<T>);

    impl<T> Visitor<'_> for ByteVisitor<T>
    where
        T: CanonicalDeserialize,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "{}", type_name::<T>())
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::deserialize_compressed(v)
                .map_err(|_| de::Error::invalid_value(de::Unexpected::Bytes(v), &self))
        }
    }

    /// Serialize a value to its compressed representation.
    pub fn serialize<V, S>(value: &V, ser: S) -> Result<S::Ok, S::Error>
    where
        V: CanonicalSerialize,
        S: Serializer,
    {
        let mut dst = Vec::new();
        value
            .serialize_compressed(&mut dst)
            .map_err(|_| S::Error::custom("serialize_compressed failed"))?;
        ser.serialize_bytes(&dst)
    }

    /// Deserialize a value from its compressed representation.
    pub fn deserialize<'de, V, D>(de: D) -> Result<V, D::Error>
    where
        V: CanonicalDeserialize,
        D: Deserializer<'de>,
    {
        de.deserialize_bytes(ByteVisitor(PhantomData))
    }
}

/// Serialize and deserialize to and from uncompressed representations.
pub mod uncompressed {
    use super::*;

    struct ByteVisitor<T>(PhantomData<T>);

    impl<T> Visitor<'_> for ByteVisitor<T>
    where
        T: CanonicalDeserialize,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "{}", type_name::<T>())
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::deserialize_uncompressed(v)
                .map_err(|_| de::Error::invalid_value(de::Unexpected::Bytes(v), &self))
        }
    }

    /// Serialize a value to its uncompressed representation.
    pub fn serialize<V, S>(value: &V, ser: S) -> Result<S::Ok, S::Error>
    where
        V: CanonicalSerialize,
        S: Serializer,
    {
        let mut dst = Vec::new();
        value
            .serialize_uncompressed(&mut dst)
            .map_err(|_| S::Error::custom("serialize_uncompressed failed"))?;
        ser.serialize_bytes(&dst)
    }

    /// Deserialize a value from its uncompressed representation.
    pub fn deserialize<'de, V, D>(de: D) -> Result<V, D::Error>
    where
        V: CanonicalDeserialize,
        D: Deserializer<'de>,
    {
        de.deserialize_bytes(ByteVisitor(PhantomData))
    }
}

#[cfg(feature = "std")]
/// Serialize and deserialize vectors of serializable values.
pub mod vec {
    use super::*;

    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct Wrapper<T>(#[serde(with = "compressed")] T)
    where
        T: CanonicalSerialize + CanonicalDeserialize;

    /// Serialize values to their compressed represnetation.
    pub fn serialize<T, S>(value: &[T], ser: S) -> Result<S::Ok, S::Error>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Copy,
        S: Serializer,
    {
        let tmp: Vec<_> = value.iter().map(|v| Wrapper(*v)).collect();
        tmp.serialize(ser)
    }

    /// Deserializes values to their compressed represnetation.
    pub fn deserialize<'de, T, D>(de: D) -> Result<Vec<T>, D::Error>
    where
        T: CanonicalSerialize + CanonicalDeserialize,
        D: Deserializer<'de>,
    {
        Vec::<Wrapper<T>>::deserialize(de).map(|w| w.into_iter().map(|w| w.0).collect())
    }
}

pub use compressed::{deserialize, serialize};

#[cfg(test)]
mod test {
    use super::*;

    use fmt::Debug;

    use ark_bls12_381::Bls12_381;
    use ark_ec::pairing::{Pairing, PairingOutput};
    use ark_ff::UniformRand;
    use bincode::config;
    use serde::{Deserialize, Serialize};

    type G1Affine = <Bls12_381 as Pairing>::G1Affine;
    type G1Projective = <Bls12_381 as Pairing>::G1;
    type G2Affine = <Bls12_381 as Pairing>::G2Affine;
    type G2Projective = <Bls12_381 as Pairing>::G2;
    type Gt = PairingOutput<Bls12_381>;
    type Scalar = <Bls12_381 as Pairing>::ScalarField;

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Wrapper<T>(#[serde(with = "super")] T)
    where
        T: CanonicalSerialize + CanonicalDeserialize + Eq + PartialEq + Debug;

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct UncommpressedWrapper<T>(#[serde(with = "uncompressed")] T)
    where
        T: CanonicalSerialize + CanonicalDeserialize + Eq + PartialEq + Debug;

    #[test]
    fn g1() {
        let mut rng = rand::thread_rng();
        let v = Wrapper(G1Affine::rand(&mut rng));

        let bin = bincode::serde::encode_to_vec(&v, config::standard()).unwrap();
        let (v_deserialized, len) =
            bincode::serde::decode_from_slice(&bin, config::standard()).unwrap();
        assert_eq!(v, v_deserialized);
        assert_eq!(bin.len(), len);
    }

    #[test]
    fn uncompressed_g1() {
        let mut rng = rand::thread_rng();
        let v = UncommpressedWrapper(G1Affine::rand(&mut rng));

        let bin = bincode::serde::encode_to_vec(&v, config::standard()).unwrap();
        let (v_deserialized, len) =
            bincode::serde::decode_from_slice(&bin, config::standard()).unwrap();
        assert_eq!(v, v_deserialized);
        assert_eq!(bin.len(), len);
    }

    #[test]
    fn g1_projective() {
        let mut rng = rand::thread_rng();
        let v = Wrapper(G1Affine::rand(&mut rng));

        let bin = bincode::serde::encode_to_vec(&v, config::standard()).unwrap();
        let (v_deserialized, len) =
            bincode::serde::decode_from_slice(&bin, config::standard()).unwrap();
        assert_eq!(v, v_deserialized);
        assert_eq!(bin.len(), len);
    }

    #[test]
    fn uncompressed_g1_projective() {
        let mut rng = rand::thread_rng();
        let v = UncommpressedWrapper(G1Projective::rand(&mut rng));

        let bin = bincode::serde::encode_to_vec(&v, config::standard()).unwrap();
        let (v_deserialized, len) =
            bincode::serde::decode_from_slice(&bin, config::standard()).unwrap();
        assert_eq!(v, v_deserialized);
        assert_eq!(bin.len(), len);
    }

    #[test]
    fn g2() {
        let mut rng = rand::thread_rng();
        let v = Wrapper(G2Affine::rand(&mut rng));

        let bin = bincode::serde::encode_to_vec(&v, config::standard()).unwrap();
        let (v_deserialized, len) =
            bincode::serde::decode_from_slice(&bin, config::standard()).unwrap();
        assert_eq!(v, v_deserialized);
        assert_eq!(bin.len(), len);
    }

    #[test]
    fn uncompressed_g2() {
        let mut rng = rand::thread_rng();
        let v = UncommpressedWrapper(G2Affine::rand(&mut rng));

        let bin = bincode::serde::encode_to_vec(&v, config::standard()).unwrap();
        let (v_deserialized, len) =
            bincode::serde::decode_from_slice(&bin, config::standard()).unwrap();
        assert_eq!(v, v_deserialized);
        assert_eq!(bin.len(), len);
    }

    #[test]
    fn g2_projective() {
        let mut rng = rand::thread_rng();
        let v = Wrapper(G2Projective::rand(&mut rng));

        let bin = bincode::serde::encode_to_vec(&v, config::standard()).unwrap();
        let (v_deserialized, len) =
            bincode::serde::decode_from_slice(&bin, config::standard()).unwrap();
        assert_eq!(v, v_deserialized);
        assert_eq!(bin.len(), len);
    }

    #[test]
    fn uncompressed_g2_projective() {
        let mut rng = rand::thread_rng();
        let v = UncommpressedWrapper(G2Projective::rand(&mut rng));

        let bin = bincode::serde::encode_to_vec(&v, config::standard()).unwrap();
        let (v_deserialized, len) =
            bincode::serde::decode_from_slice(&bin, config::standard()).unwrap();
        assert_eq!(v, v_deserialized);
        assert_eq!(bin.len(), len);
    }

    #[test]
    fn gt() {
        let mut rng = rand::thread_rng();
        let v = Wrapper(Gt::rand(&mut rng));

        let bin = bincode::serde::encode_to_vec(&v, config::standard()).unwrap();
        let (v_deserialized, len) =
            bincode::serde::decode_from_slice(&bin, config::standard()).unwrap();
        assert_eq!(v, v_deserialized);
        assert_eq!(bin.len(), len);
    }

    #[test]
    fn uncompressed_gt() {
        let mut rng = rand::thread_rng();
        let v = UncommpressedWrapper(Gt::rand(&mut rng));

        let bin = bincode::serde::encode_to_vec(&v, config::standard()).unwrap();
        let (v_deserialized, len) =
            bincode::serde::decode_from_slice(&bin, config::standard()).unwrap();
        assert_eq!(v, v_deserialized);
        assert_eq!(bin.len(), len);
    }

    #[test]
    fn scalar() {
        let mut rng = rand::thread_rng();
        let v = Wrapper(Scalar::rand(&mut rng));
        let v_uncompressed = UncommpressedWrapper(v.0);

        let bin = bincode::serde::encode_to_vec(&v, config::standard()).unwrap();
        assert_eq!(
            bin,
            bincode::serde::encode_to_vec(&v_uncompressed, config::standard()).unwrap()
        );

        let (v_deserialized, len) =
            bincode::serde::decode_from_slice(&bin, config::standard()).unwrap();
        assert_eq!(v, v_deserialized);
        assert_eq!(bin.len(), len);

        let (v_deserialized, len) =
            bincode::serde::decode_from_slice(&bin, config::standard()).unwrap();
        assert_eq!(v_uncompressed, v_deserialized);
        assert_eq!(bin.len(), len);
    }
}

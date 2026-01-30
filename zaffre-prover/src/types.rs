//! core types for zaffre prover

use borsh::{BorshDeserialize, BorshSerialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct Secret([u8; 32]);

impl Secret {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Commitment(pub [u8; 32]);

impl Commitment {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl BorshSerialize for Commitment {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshSerialize::serialize(&self.0, writer)
    }
}

impl BorshDeserialize for Commitment {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(Self(<[u8; 32]>::deserialize_reader(reader)?))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Nullifier(pub [u8; 32]);

impl Nullifier {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl BorshSerialize for Nullifier {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshSerialize::serialize(&self.0, writer)
    }
}

impl BorshDeserialize for Nullifier {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(Self(<[u8; 32]>::deserialize_reader(reader)?))
    }
}

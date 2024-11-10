use ::starknet::core::{
    crypto::{ExtendedSignature as StarknetExtendedSignature, Signature as StarknetSignature},
    types::Felt,
};
use rsa::{RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub enum AccountType {
    OpenZeppelin,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FeltWrapper(#[serde(with = "felt_serde")] pub Felt);

pub struct RSAKeyPair {
    pub private: RsaPrivateKey,
    pub public: RsaPublicKey,
}

// Custom serialization module for Felt
mod felt_serde {
    use super::*;
    use serde::de::{self, Visitor};
    use serde::{Deserializer, Serializer};
    use std::fmt;

    pub fn serialize<S>(felt: &Felt, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = felt.to_bytes_be();
        serializer.serialize_bytes(&bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Felt, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FeltVisitor;

        impl<'de> Visitor<'de> for FeltVisitor {
            type Value = Felt;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a byte array representing a Felt")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v.len() != 32 {
                    return Err(E::custom(format!(
                        "expected 32 bytes for Felt, got {}",
                        v.len()
                    )));
                }
                let mut arr = [0u8; 32];
                arr.copy_from_slice(v);
                Ok(Felt::from_bytes_be(&arr))
            }
        }

        deserializer.deserialize_bytes(FeltVisitor)
    }
}

// Update ECDSASignature to use FeltWrapper
#[derive(Debug, Clone, Copy)]
pub struct ECDSASignature {
    pub r: FeltWrapper,
    pub s: FeltWrapper,
    pub v: FeltWrapper,
}

impl ECDSASignature {
    /// Converts the signature into a single Felt by concatenating r, s, and v
    /// The format will be: r_low || s_low || v
    pub fn to_felt(&self) -> Result<Felt, &'static str> {
        // Get the underlying Felt values
        let r: Felt = self.r.into();
        let s: Felt = self.s.into();
        let v: Felt = self.v.into();

        // Convert to bytes in big-endian format
        let r_bytes = r.to_bytes_be();
        let s_bytes = s.to_bytes_be();
        let v_bytes = v.to_bytes_be();

        // Create a buffer to hold the concatenated values
        let mut buffer = Vec::with_capacity(32);

        // Take the last 10 bytes of r
        buffer.extend_from_slice(&r_bytes[22..]);
        // Take the last 10 bytes of s
        buffer.extend_from_slice(&s_bytes[22..]);
        // Take the last byte of v
        buffer.push(v_bytes[31]);

        // Pad with zeros to get 32 bytes
        while buffer.len() < 32 {
            buffer.insert(0, 0);
        }

        // Convert back to Felt
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&buffer);
        Ok(Felt::from_bytes_be(&arr))
    }

    /// Reconstructs an ECDSASignature from a Felt
    pub fn from_felt(felt: Felt) -> Result<Self, &'static str> {
        let bytes = felt.to_bytes_be();

        // Need at least 21 bytes for the signature components
        if bytes.len() != 32 {
            return Err("Invalid felt length for signature");
        }

        // Extract the components
        // Last byte is v
        let v_byte = bytes[31];

        // Previous 10 bytes are s
        let mut s_bytes = [0u8; 32];
        s_bytes[22..].copy_from_slice(&bytes[21..31]);

        // Previous 10 bytes are r
        let mut r_bytes = [0u8; 32];
        r_bytes[22..].copy_from_slice(&bytes[11..21]);

        Ok(ECDSASignature {
            r: FeltWrapper(Felt::from_bytes_be(&r_bytes)),
            s: FeltWrapper(Felt::from_bytes_be(&s_bytes)),
            v: FeltWrapper(Felt::from(v_byte as u64)),
        })
    }
}

impl From<Felt> for FeltWrapper {
    fn from(felt: Felt) -> Self {
        FeltWrapper(felt)
    }
}

impl From<FeltWrapper> for Felt {
    fn from(wrapper: FeltWrapper) -> Self {
        wrapper.0
    }
}

impl From<&Felt> for FeltWrapper {
    fn from(felt: &Felt) -> Self {
        FeltWrapper(felt.clone())
    }
}

impl From<&FeltWrapper> for Felt {
    fn from(wrapper: &FeltWrapper) -> Self {
        wrapper.0.clone()
    }
}

impl PartialEq for FeltWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialEq for ECDSASignature {
    fn eq(&self, other: &Self) -> bool {
        self.r == other.r && self.s == other.s && self.v == other.v
    }
}

/// From trait for the Starknet ExtendedSignature type
impl From<StarknetExtendedSignature> for ECDSASignature {
    fn from(value: StarknetExtendedSignature) -> Self {
        ECDSASignature {
            r: FeltWrapper(value.r),
            s: FeltWrapper(value.s),
            v: FeltWrapper(value.v),
        }
    }
}

/// Into trait for the Starknet ExtendedSignature type
impl Into<StarknetExtendedSignature> for ECDSASignature {
    fn into(self: ECDSASignature) -> StarknetExtendedSignature {
        StarknetExtendedSignature {
            r: self.r.into(),
            s: self.s.into(),
            v: self.v.into(),
        }
    }
}

// Into trait for the Starknet Signature type
impl Into<StarknetSignature> for ECDSASignature {
    fn into(self: ECDSASignature) -> StarknetSignature {
        StarknetSignature {
            r: self.r.into(),
            s: self.s.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_felt_conversion() {
        // Create a test signature
        let original_sig = ECDSASignature {
            r: FeltWrapper(Felt::from_hex("0x1234567890").unwrap()),
            s: FeltWrapper(Felt::from_hex("0xabcdef1234").unwrap()),
            v: FeltWrapper(Felt::from(1)),
        };

        // Convert to felt
        let felt = original_sig.to_felt().unwrap();

        // Convert back to signature
        let recovered_sig = ECDSASignature::from_felt(felt).unwrap();

        // Verify equality
        assert_eq!(original_sig, recovered_sig);
    }

    #[test]
    fn test_signature_components() {
        let sig = ECDSASignature {
            r: FeltWrapper(Felt::from_hex("0x1234567890").unwrap()),
            s: FeltWrapper(Felt::from_hex("0xabcdef1234").unwrap()),
            v: FeltWrapper(Felt::from(1)),
        };

        let felt = sig.to_felt().unwrap();
        let recovered = ECDSASignature::from_felt(felt).unwrap();

        assert_eq!(sig.r, recovered.r);
        assert_eq!(sig.s, recovered.s);
        assert_eq!(sig.v, recovered.v);
    }
}

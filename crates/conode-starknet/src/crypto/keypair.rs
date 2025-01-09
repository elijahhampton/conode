use bip39::{Language, Mnemonic, Seed};
use conode_types::crypto::AccountType;
use hdpath::{Purpose, StandardHDPath};
use lazy_static::lazy_static;
use libp2p::identity::{self, ed25519};
use libp2p::PeerId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use starknet::core::types::Felt;
use starknet::core::utils::{get_contract_address};
use starknet::signers::SigningKey;
use starknet_crypto::get_public_key;
use std::collections::HashMap;
use std::error::Error;
use tiny_keccak::{Hasher, Keccak};

// Well known account hashes from OpenZeppelin.
lazy_static! {
    pub static ref ACCOUNT_CLASS_HASHES: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        // Hash for Open Zeppelin Account Contract v0.6.1
        m.insert(
            "OZ_0.6.1",
            "0x04c6d6cf894f8bc96bb9c525e6853e5483177841f7388f74c3b91df4579e662e"
        );
        m
    };
}

/// Represents all of the necessary keypairs the node needs for
/// signing, verifying, encryption and decryption along with the starknet keypair mnemonic
/// and the node's starknet address.
#[derive(Clone, Debug)]
pub struct KeyPair {
    keypair: ed25519::Keypair, // ed25519 Keypair
    pub starknet_address: Felt,
    mnemonic: Option<Mnemonic>, // Mnemonic representing the seed used to generate the private keys
    stark_private_key: Felt,    // Starknet private key
    stark_public_key: Felt,     // Starknet public key
}

impl KeyPair {
    /// Generates a new KeyPair with fresh random entropy.
    /// Creates both Ed25519 and STARK keypairs from the same seed.
    ///
    /// # Returns
    /// * A new KeyPair instance with generated keys and derived Starknet address
    pub fn generate() -> Self {
        // Generate random entropy for the mnemonic
        let mut entropy = [0u8; 32];
        getrandom::getrandom(&mut entropy).unwrap();
        let mnemonic = Mnemonic::from_entropy(&entropy, Language::English).unwrap();

        Self::from_mnemonic_internal(mnemonic)
    }

    fn from_mnemonic_internal(mnemonic: Mnemonic) -> Self {
        let seed = Seed::new(&mnemonic, "");
        let seed_bytes = seed.as_bytes();

        // Define standard BIP44 paths for the ed25519 and starknet keypair
        let ed25519_path = StandardHDPath::new(Purpose::Pubkey, 0, 0, 0, 0);
        let stark_path = StandardHDPath::new(Purpose::Pubkey, 0, 0, 0, 1);

        // Generate STARK private key using BIP44
        let mut hasher = Keccak::v256();
        hasher.update(stark_path.to_string().as_bytes());
        hasher.update(seed_bytes);
        let mut stark_key_bytes = [0u8; 32];
        hasher.finalize(&mut stark_key_bytes);

        let stark_private_key = Felt::from_bytes_be(&stark_key_bytes);
        let stark_public_key = Self::derive_stark_public_key(&stark_private_key);

        // Generate Ed25519 keypair using BIP44
        let mut hasher = Keccak::v256();
        hasher.update(ed25519_path.to_string().as_bytes());
        hasher.update(seed_bytes);
        let mut ed25519_key_bytes = [0u8; 32];
        hasher.finalize(&mut ed25519_key_bytes);

        let secret = ed25519::SecretKey::try_from_bytes(ed25519_key_bytes)
            .expect("Valid key bytes should generate valid secret key");
        let keypair = ed25519::Keypair::from(secret);

        let starknet_address =
            Self::derive_starknet_address(&stark_public_key, AccountType::OpenZeppelin, "OZ_0.6.1")
                .unwrap_or_else(|_| Felt::from_bytes_be(&[0u8; 32]));

        Self {
            keypair,
            stark_private_key,
            stark_public_key,
            starknet_address,
            mnemonic: Some(mnemonic),
        }
    }

    /// Creates a KeyPair from an existing mnemonic phrase.
    ///
    /// # Arguments
    /// * `mnemonic` - BIP39 mnemonic phrase as string
    ///
    /// # Returns
    /// * Result containing new KeyPair or error if mnemonic is invalid
    pub fn from_mnemonic(mnemonic: &str) -> Result<Self, Box<dyn Error>> {
        let mnemonic = Mnemonic::from_phrase(mnemonic, Language::English)?;
        Ok(Self::from_mnemonic_internal(mnemonic))
    }

    /// Returns a reference to the mnemonic used to create the account.
    ///
    /// # Returns
    /// * Optional reference to the Mnemonic if it exists
    pub fn mnemonic(&self) -> Option<&Mnemonic> {
        self.mnemonic.as_ref()
    }

    /// Derives a Starknet public key from a private key.
    ///
    /// # Arguments
    /// * `private_key` - STARK curve private key
    ///
    /// # Returns
    /// * Corresponding STARK curve public key
    fn derive_stark_public_key(private_key: &Felt) -> Felt {
        get_public_key(private_key)
    }

    /// Derives a Starknet address from a public key.
    ///
    /// # Arguments
    /// * `public_key` - Ed25519 public key
    /// * `account_type` - Type of Starknet account (e.g., OpenZeppelin)
    /// * `version` - Version string for the account implementation
    ///
    /// # Returns
    /// * Result containing derived Starknet address or error string
    pub fn derive_starknet_address(
        public_key: &Felt,
        account_type: AccountType,
        version: &str,
    ) -> Result<Felt, &'static str> {
        // Verify we're using a known good class hash
        let class_hash = match ACCOUNT_CLASS_HASHES.get(version) {
            Some(&hash_str) => Felt::from_hex(hash_str).map_err(|_| "Invalid class hash format")?,
            None => return Err("Unknown account version"),
        };

        let salt = Self::derive_salt(public_key, 0);
        let constructor_calldata = vec![*public_key];
        let deployer_address = Felt::ZERO;

        Ok(get_contract_address(
            salt,
            class_hash,
            &constructor_calldata,
            deployer_address,
        ))
    }

    pub fn get_stark_signing_key(&self) -> SigningKey {
        SigningKey::from_secret_scalar(self.stark_private_key)
    }

    /// Derives a salt value from a public key and index.
    ///
    /// # Arguments
    /// * `public_key` - Public key as Felt
    /// * `index` - Key index
    ///
    /// # Returns
    /// * Derived salt value as Felt
    fn derive_salt(caller_address: &Felt, salt: u128) -> Felt {
        let mut hasher = Keccak::v256();
        hasher.update(&caller_address.to_bytes_be());
        hasher.update(&salt.to_be_bytes());
        let mut result = [0u8; 32];
        hasher.finalize(&mut result);
        Felt::from_bytes_be(&result)
    }

    /// Returns a reference to the starknet private key.
    pub fn stark_private_key(&self) -> &Felt {
        &self.stark_private_key
    }

    /// Returns a reference to the starknet public key.
    pub fn stark_public_key(&self) -> &Felt {
        &self.stark_public_key
    }

    /// Retrieves the class hash for a given account type and version.
    ///
    /// # Arguments
    /// * `account_type` - Type of account (e.g., OpenZeppelin)
    /// * `version` - Version string of the account implementation
    ///
    /// # Returns
    /// * Result containing class hash as Felt or error string
    fn get_class_hash(account_type: AccountType, version: &str) -> Result<Felt, &'static str> {
        let key = match account_type {
            AccountType::OpenZeppelin => {
                format!("OZ_{}", version)
            }
        };

        let hash_str = ACCOUNT_CLASS_HASHES
            .get(key.as_str())
            .or_else(|| ACCOUNT_CLASS_HASHES.get("OZ_0.6.1"))
            .ok_or("Unknown account type or version")?;

        Felt::from_hex(hash_str).map_err(|_| "Invalid hash format")
    }

    /// Returns the public key portion of the keypair.
    ///
    /// # Returns
    /// * PublicKey wrapper around the Ed25519 public key
    pub fn public_key(&self) -> PublicKey {
        PublicKey {
            public_key: self.keypair.public(),
        }
    }

    /// Derives the peer ID from the public key.
    ///
    /// # Returns
    /// * libp2p PeerId derived from the Ed25519 public key
    pub fn peer_id(&self) -> PeerId {
        PeerId::from_public_key(&identity::PublicKey::from(self.keypair.public()))
    }

    /// Returns the keypair as a byte vector.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.keypair.to_bytes().to_vec()
    }

    /// Returns a reference to the Ed25519 keypair.
    pub fn keypair(&self) -> &ed25519::Keypair {
        &self.keypair
    }
}

/// Wrapper for Ed25519 public key with serialization support.
#[derive(Clone, Debug)]
pub struct PublicKey {
    /// The underlying Ed25519 public key
    public_key: ed25519::PublicKey,
}

impl PublicKey {
    /// Returns the public key as a byte vector.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.public_key.to_bytes().to_vec()
    }

    /// Returns a reference to the underlying Ed25519 public key.
    pub fn public_key(&self) -> &ed25519::PublicKey {
        &self.public_key
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.public_key().to_bytes())
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
        let public_key =
            ed25519::PublicKey::try_from_bytes(&bytes).map_err(serde::de::Error::custom)?;
        Ok(PublicKey { public_key })
    }
}

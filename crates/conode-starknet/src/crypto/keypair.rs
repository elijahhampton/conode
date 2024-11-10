use bip39::{Language, Mnemonic, Seed};
use conode_types::crypto::AccountType;
use lazy_static::lazy_static;
use libp2p::identity::{self, ed25519};
use libp2p::PeerId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use starknet::core::types::Felt;
use starknet::core::utils::{get_contract_address, starknet_keccak};
use starknet_crypto::get_public_key;
use std::collections::HashMap;
use std::error::Error;

// Well known account hashes primarly from OpenZeppelin. We use these account hashes to create accounts
// for the node if a new account is requested.
lazy_static! {
    pub static ref ACCOUNT_CLASS_HASHES: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        // Hash for OZ 0.6.1 compiled with Cairo 0.10.3
        m.insert(
            "OZ_0.6.1",
            "0x508fc648f7dc864be1242384cc819f0d23bfeea97b5216923ab769e103c9775"
        );
        // Hash for OZ accounts from Nile (Cairo 0.10.1)
        m.insert(
            "OZ_0.5.0",
            "0x058d97f7d76e78f44905cc30cb65b91ea49a4b908a76703c54197bca90f81773"
        );
        m
    };
}

/// A structure representing all of the necessary keypairs the node needs for
/// signing, verifying, encryption and decryption along with the starknet keypair mnemonic
/// and the node's starknet address.
#[derive(Clone, Debug)]
pub struct KeyPair {
    keypair: ed25519::Keypair,
    pub starknet_address: Felt,
    mnemonic: Option<Mnemonic>,
    // STARK curve operations
    stark_private_key: Felt,
    stark_public_key: Felt,
}

impl KeyPair {
    /// Generates a new KeyPair with fresh random entropy.
    /// Creates both Ed25519 and STARK keypairs from the same seed.
    ///
    /// # Returns
    /// * A new KeyPair instance with generated keys and derived Starknet address
    pub fn generate() -> Self {
        // Generate random entropy for the mnemonic.
        let mut entropy = [0u8; 32];
        getrandom::getrandom(&mut entropy).unwrap();

        // Generate mnemonic from entropy
        let mnemonic = Mnemonic::from_entropy(&entropy, Language::English).unwrap();

        // Derive seed
        let seed = Seed::new(&mnemonic, "");
        let seed_bytes = seed.as_bytes();

        // Generate Ed25519 keypair from first 32 bytes
        let mut ed25519_key_bytes = [0u8; 32];
        ed25519_key_bytes.copy_from_slice(&seed_bytes[..32]);

        let secret = ed25519::SecretKey::try_from_bytes(ed25519_key_bytes)
            .expect("Valid key bytes should generate valid secret key");
        let keypair = ed25519::Keypair::from(secret);

        // Generate STARK keypair from next 32 bytes
        let mut stark_key_bytes = [0u8; 32];
        stark_key_bytes.copy_from_slice(&seed_bytes[32..64]);
        let stark_private_key = Felt::from_bytes_be(&stark_key_bytes);

        // Derive STARK public key using starknet_crypto
        let stark_public_key = Self::derive_stark_public_key(&stark_private_key);

        // Derive Starknet address using Ed25519 public key (maintaining existing behavior)
        let starknet_address =
            Self::derive_starknet_address(&keypair.public(), AccountType::OpenZeppelin, "OZ_0.6.1")
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
        let mnemonic = match Mnemonic::from_phrase(mnemonic, Language::English) {
            Ok(m) => m,
            Err(e) => {
                return Err(Box::new(e));
            }
        };

        // @dev Password based seeds are currently not supported
        let seed = Seed::new(&mnemonic, "");
        let seed_bytes = seed.as_bytes();

        // Generate Ed25519 keypair
        let mut ed25519_key_bytes = [0u8; 32];
        ed25519_key_bytes.copy_from_slice(&seed_bytes[..32]);

        let secret = match ed25519::SecretKey::try_from_bytes(ed25519_key_bytes) {
            Ok(s) => s,
            Err(e) => {
                return Err(Box::new(e));
            }
        };

        let keypair = ed25519::Keypair::from(secret);

        // Generate STARK keypair
        let mut stark_key_bytes = [0u8; 32];
        stark_key_bytes.copy_from_slice(&seed_bytes[32..64]);
        let stark_private_key = Felt::from_bytes_be(&stark_key_bytes);
        let stark_public_key = Self::derive_stark_public_key(&stark_private_key);

        match Self::derive_starknet_address(
            &keypair.public(),
            AccountType::OpenZeppelin,
            "OZ_0.6.1",
        ) {
            Ok(address) => Ok(Self {
                keypair,
                stark_private_key,
                stark_public_key,
                starknet_address: address,
                mnemonic: Some(mnemonic),
            }),
            Err(e) => Err(e.into()),
        }
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
        public_key: &ed25519::PublicKey,
        account_type: AccountType,
        version: &str,
    ) -> Result<Felt, &'static str> {
        let public_key_bytes = public_key.to_bytes();
        let public_key_felt = Felt::from_bytes_be(&public_key_bytes);

        let salt = Self::derive_salt(&public_key_felt, 0);

        let class_hash = match Self::get_class_hash(account_type.clone(), version) {
            Ok(hash) => hash,
            Err(e) => {
                return Err(e);
            }
        };

        let constructor_calldata = vec![public_key_felt];

        let deployer_address = Felt::ZERO;

        // Make sure we're using the correct class hash for OpenZeppelin Account Contract v0.6.1
        lazy_static! {
            static ref KNOWN_GOOD_HASH: &'static str =
                "0x04c6d6cf894f8bc96bb9c525e6853e5483177841f7388f74c3b91df4579e662e";
        }

        let contract_address =
            get_contract_address(salt, class_hash, &constructor_calldata, deployer_address);

        Ok(contract_address)
    }

    /// Derives a salt value from a public key and index.
    ///
    /// # Arguments
    /// * `public_key` - Public key as Felt
    /// * `index` - Key index
    ///
    /// # Returns
    /// * Derived salt value as Felt
    fn derive_salt(public_key: &Felt, index: u64) -> Felt {
        let mut data = Vec::new();
        data.extend_from_slice(&public_key.to_bytes_be());
        data.extend_from_slice(&index.to_be_bytes());
        starknet_keccak(&data)
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

        // Try exact version first
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

/// Wrapper struct for Ed25519 public key with serialization support.
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

use rand::rngs::OsRng;
use rsa::{
    pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey},
    Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey,
};

pub struct RsaKeypair {
    private: RsaPrivateKey,
    public: RsaPublicKey,
}

impl RsaKeypair {
    // Generate new keypair
    pub fn generate() -> Result<Self, Box<dyn std::error::Error>> {
        let mut rng = OsRng;
        let private = RsaPrivateKey::new(&mut rng, 2048)?;
        let public = RsaPublicKey::from(&private);
        Ok(Self { private, public })
    }

    // Get public key for sharing
    pub fn public_key(&self) -> &RsaPublicKey {
        &self.public
    }

    // Encrypt with public key
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut rng = OsRng;
        Ok(self.public.encrypt(&mut rng, Pkcs1v15Encrypt, data)?)
    }

    // Decrypt with private key
    pub fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(self.private.decrypt(Pkcs1v15Encrypt, encrypted_data)?)
    }

    // Serialize private key to bytes
    pub fn private_key_to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(self.private.to_pkcs8_der()?.as_bytes().to_vec())
    }

    // Serialize public key to bytes
    pub fn public_key_to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(self.public.to_public_key_der()?.as_bytes().to_vec())
    }

    // Load private key from bytes
    pub fn private_key_from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let private = RsaPrivateKey::from_pkcs8_der(bytes)?;
        let public = RsaPublicKey::from(&private);
        Ok(Self { private, public })
    }

    // Load public key from bytes
    pub fn public_key_from_bytes(bytes: &[u8]) -> Result<RsaPublicKey, Box<dyn std::error::Error>> {
        RsaPublicKey::from_public_key_der(bytes).map_err(Into::into)
    }
}

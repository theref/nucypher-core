use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};
use umbral_pre::{
    decrypt_original, encrypt, Capsule, EncryptionError, PublicKey, SecretKey, SerializableToArray,
    Signature, Signer, VerifiedKeyFrag,
};

use crate::address::Address;
use crate::hrac::HRAC;
use crate::key_frag::EncryptedKeyFrag;
use crate::versioning::{
    messagepack_deserialize, messagepack_serialize, ProtocolObject, ProtocolObjectInner,
};

/// A structure containing `KeyFrag` objects encrypted for Ursulas chosen for this policy.
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct TreasureMap {
    /// Threshold for successful re-encryption.
    pub threshold: u8,
    /// Policy HRAC.
    pub hrac: HRAC,
    // TODO: HashMap requires `std`. Do we actually want `no_std` for this crate?
    // There seems to be a BTreeMap available for no_std environments,
    // but let's just use vector for now.
    /// Encrypted key frags assigned to target Ursulas.
    pub destinations: Vec<(Address, EncryptedKeyFrag)>,
    /// A key to create encrypted messages under this policy.
    pub policy_encrypting_key: PublicKey,
    /// Publisher's verifying key.
    pub publisher_verifying_key: PublicKey,
}

impl TreasureMap {
    /// Create a new treasure map for a collection of ursulas and kfrags.
    ///
    /// Panics if `threshold` is set to 0,
    /// or the number of assigned keyfrags is less than `threshold`.
    pub fn new(
        signer: &Signer,
        hrac: &HRAC,
        policy_encrypting_key: &PublicKey,
        // TODO: would be nice to enforce that checksum addresses are not repeated,
        // but there is no "map-like" trait in Rust, and a specific map class seems too restrictive...
        assigned_kfrags: &[(Address, PublicKey, VerifiedKeyFrag)],
        threshold: u8,
    ) -> Self {
        // Panic here since violation of these conditions indicates a bug on the caller's side.
        assert!(threshold != 0, "threshold must be non-zero");
        assert!(
            assigned_kfrags.len() >= threshold as usize,
            "threshold cannot be larger than the total number of shares"
        );

        // Encrypt each kfrag for an Ursula.
        let mut destinations = Vec::new();
        for (ursula_checksum_address, ursula_encrypting_key, verified_kfrag) in
            assigned_kfrags.iter()
        {
            let encrypted_kfrag =
                EncryptedKeyFrag::new(signer, ursula_encrypting_key, hrac, verified_kfrag).unwrap();
            destinations.push((*ursula_checksum_address, encrypted_kfrag));
        }

        Self {
            threshold,
            hrac: *hrac,
            destinations,
            policy_encrypting_key: *policy_encrypting_key,
            publisher_verifying_key: signer.verifying_key(),
        }
    }

    /// Encrypts the treasure map for Bob.
    pub fn encrypt(
        &self,
        signer: &Signer,
        recipient_key: &PublicKey,
    ) -> Result<EncryptedTreasureMap, EncryptionError> {
        EncryptedTreasureMap::new(signer, recipient_key, self)
    }
}

impl<'a> ProtocolObjectInner<'a> for TreasureMap {
    fn brand() -> [u8; 4] {
        *b"TMap"
    }

    fn version() -> (u16, u16) {
        (1, 0)
    }

    fn unversioned_to_bytes(&self) -> Box<[u8]> {
        messagepack_serialize(&self)
    }

    fn unversioned_from_bytes(minor_version: u16, bytes: &[u8]) -> Option<Result<Self, String>> {
        if minor_version == 0 {
            Some(messagepack_deserialize(bytes))
        } else {
            None
        }
    }
}

impl<'a> ProtocolObject<'a> for TreasureMap {}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
struct AuthorizedTreasureMap {
    signature: Signature,
    treasure_map: TreasureMap,
}

impl AuthorizedTreasureMap {
    fn new(signer: &Signer, recipient_key: &PublicKey, treasure_map: &TreasureMap) -> Self {
        let mut message = recipient_key.to_array().to_vec();
        message.extend(treasure_map.to_bytes().iter());

        let signature = signer.sign(&message);

        Self {
            signature,
            treasure_map: treasure_map.clone(),
        }
    }

    fn verify(
        &self,
        recipient_key: &PublicKey,
        publisher_verifying_key: &PublicKey,
    ) -> Option<TreasureMap> {
        let mut message = recipient_key.to_array().to_vec();
        message.extend(self.treasure_map.to_bytes().iter());

        if !self.signature.verify(publisher_verifying_key, &message) {
            return None;
        }
        Some(self.treasure_map.clone())
    }
}

impl<'a> ProtocolObjectInner<'a> for AuthorizedTreasureMap {
    fn brand() -> [u8; 4] {
        *b"AMap"
    }

    fn version() -> (u16, u16) {
        (1, 0)
    }

    fn unversioned_to_bytes(&self) -> Box<[u8]> {
        messagepack_serialize(&self)
    }

    fn unversioned_from_bytes(minor_version: u16, bytes: &[u8]) -> Option<Result<Self, String>> {
        if minor_version == 0 {
            Some(messagepack_deserialize(bytes))
        } else {
            None
        }
    }
}

impl<'a> ProtocolObject<'a> for AuthorizedTreasureMap {}

/// A treasure map encrypted for Bob.
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedTreasureMap {
    capsule: Capsule,
    ciphertext: Box<[u8]>,
}

impl EncryptedTreasureMap {
    fn new(
        signer: &Signer,
        recipient_key: &PublicKey,
        treasure_map: &TreasureMap,
    ) -> Result<Self, EncryptionError> {
        // TODO: using Umbral for encryption to avoid introducing more crypto primitives.
        // Most probably it is an overkill, unless it can be used somehow
        // for Ursula-to-Ursula "baton passing".

        // TODO: `publisher` here can be different from the one in TreasureMap, it seems.
        // Do we ever cross-check them? Do we want to enforce them to be the same?

        let authorized_tmap = AuthorizedTreasureMap::new(signer, recipient_key, treasure_map);
        let (capsule, ciphertext) = encrypt(recipient_key, &authorized_tmap.to_bytes())?;

        Ok(Self {
            capsule,
            ciphertext,
        })
    }

    /// Decrypts and verifies the treasure map.
    pub fn decrypt(
        &self,
        sk: &SecretKey,
        publisher_verifying_key: &PublicKey,
    ) -> Option<TreasureMap> {
        let plaintext = decrypt_original(sk, &self.capsule, &self.ciphertext).unwrap();
        let auth_tmap = AuthorizedTreasureMap::from_bytes(&plaintext).unwrap();
        auth_tmap.verify(&sk.public_key(), publisher_verifying_key)
    }
}

impl<'a> ProtocolObjectInner<'a> for EncryptedTreasureMap {
    fn brand() -> [u8; 4] {
        *b"EMap"
    }

    fn version() -> (u16, u16) {
        (1, 0)
    }

    fn unversioned_to_bytes(&self) -> Box<[u8]> {
        messagepack_serialize(&self)
    }

    fn unversioned_from_bytes(minor_version: u16, bytes: &[u8]) -> Option<Result<Self, String>> {
        if minor_version == 0 {
            Some(messagepack_deserialize(bytes))
        } else {
            None
        }
    }
}

impl<'a> ProtocolObject<'a> for EncryptedTreasureMap {}

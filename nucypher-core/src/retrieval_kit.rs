use alloc::boxed::Box;
use alloc::collections::BTreeSet;
use alloc::string::String;
use core::iter::FromIterator;

use serde::{Deserialize, Serialize};
use umbral_pre::Capsule;

use crate::address::Address;
use crate::message_kit::MessageKit;
use crate::versioning::{
    messagepack_deserialize, messagepack_serialize, ProtocolObject, ProtocolObjectInner,
};

/// An object encapsulating the information necessary for retrieval of cfrags from Ursulas.
/// Contains the capsule and the checksum addresses of Ursulas from which the requester
/// already received cfrags.
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct RetrievalKit {
    /// The ciphertext's capsule.
    pub capsule: Capsule,
    /// The addresses that have already been queried for reencryption.
    pub queried_addresses: BTreeSet<Address>,
}

impl RetrievalKit {
    /// Creates a new retrival kit from a message kit.
    pub fn from_message_kit(message_kit: &MessageKit) -> Self {
        Self {
            capsule: message_kit.capsule,
            queried_addresses: BTreeSet::<Address>::new(),
        }
    }

    /// Creates a new retrieval kit recording the addresses already queried for reencryption.
    pub fn new<'a, I>(capsule: &Capsule, queried_addresses: I) -> Self
    where
        I: Iterator<Item = &'a Address>,
    {
        // Can store cfrags too, if we're worried about Ursulas supplying duplicate ones.
        Self {
            capsule: *capsule,
            queried_addresses: BTreeSet::from_iter(queried_addresses.cloned()),
        }
    }
}

impl<'a> ProtocolObjectInner<'a> for RetrievalKit {
    fn brand() -> [u8; 4] {
        *b"RKit"
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

impl<'a> ProtocolObject<'a> for RetrievalKit {}

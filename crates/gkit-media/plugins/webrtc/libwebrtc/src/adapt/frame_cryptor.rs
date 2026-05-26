//! FrameCryptor (E2EE) adapter wrapping `libwebrtc::native::frame_cryptor`.
//!
//! LiveKit's libwebrtc fork exposes a FrameCryptor API for end-to-end encryption
//! of media tracks. This module provides thin wrappers around:
//!
//! - [`KeyProvider`] – manages encryption keys per participant
//! - [`FrameCryptor`] – attaches to an `RtpSender` or `RtpReceiver` to encrypt /
//!   decrypt media frames
//! - [`DataPacketCryptor`] – encrypts / decrypts data channel payloads
//!
//! The underlying C++ FFI is in [`webrtc_sys::frame_cryptor`].

use libwebrtc::native::frame_cryptor as lk_fc;
use libwebrtc::rtp_receiver::RtpReceiver;
use libwebrtc::rtp_sender::RtpSender;

// ---------------------------------------------------------------------------
// Re-exports of libwebrtc types
// ---------------------------------------------------------------------------

/// Algorithm used for key derivation.
pub use lk_fc::KeyDerivationAlgorithm;

/// Symmetric encryption algorithm for media frames.
pub use lk_fc::EncryptionAlgorithm;

/// Current encryption / decryption state for a participant.
pub use lk_fc::EncryptionState;

/// An encrypted frame / data packet.
pub use lk_fc::EncryptedPacket;

/// Callback for FrameCryptor state changes.
pub use lk_fc::OnStateChange;

// ---------------------------------------------------------------------------
// KeyProvider
// ---------------------------------------------------------------------------

/// Options for creating a [`KeyProvider`].
#[derive(Debug, Clone)]
pub struct KeyProviderOptions {
    pub shared_key: bool,
    pub ratchet_window_size: i32,
    pub ratchet_salt: Vec<u8>,
    pub failure_tolerance: i32,
    pub key_ring_size: i32,
    pub key_derivation_algorithm: KeyDerivationAlgorithm,
}

impl From<KeyProviderOptions> for lk_fc::KeyProviderOptions {
    fn from(opts: KeyProviderOptions) -> Self {
        lk_fc::KeyProviderOptions {
            shared_key: opts.shared_key,
            ratchet_window_size: opts.ratchet_window_size,
            ratchet_salt: opts.ratchet_salt,
            failure_tolerance: opts.failure_tolerance,
            key_ring_size: opts.key_ring_size,
            key_derivation_algorithm: opts.key_derivation_algorithm,
        }
    }
}

/// Manages encryption keys for E2EE.
///
/// Wraps [`lk_fc::KeyProvider`] and is shared between [`FrameCryptor`] and
/// [`DataPacketCryptor`] instances.
pub struct LkKeyProvider {
    inner: lk_fc::KeyProvider,
}

impl LkKeyProvider {
    pub fn new(options: KeyProviderOptions) -> Self {
        Self {
            inner: lk_fc::KeyProvider::new(options.into()),
        }
    }

    pub fn set_shared_key(&self, key_index: i32, key: Vec<u8>) -> bool {
        self.inner.set_shared_key(key_index, key)
    }

    pub fn ratchet_shared_key(&self, key_index: i32) -> Option<Vec<u8>> {
        self.inner.ratchet_shared_key(key_index)
    }

    pub fn get_shared_key(&self, key_index: i32) -> Option<Vec<u8>> {
        self.inner.get_shared_key(key_index)
    }

    pub fn set_key(&self, participant_id: String, key_index: i32, key: Vec<u8>) -> bool {
        self.inner.set_key(participant_id, key_index, key)
    }

    pub fn ratchet_key(&self, participant_id: String, key_index: i32) -> Option<Vec<u8>> {
        self.inner.ratchet_key(participant_id, key_index)
    }

    pub fn get_key(&self, participant_id: String, key_index: i32) -> Option<Vec<u8>> {
        self.inner.get_key(participant_id, key_index)
    }

    pub fn set_sif_trailer(&self, trailer: Vec<u8>) {
        self.inner.set_sif_trailer(trailer);
    }
}

// ---------------------------------------------------------------------------
// FrameCryptor
// ---------------------------------------------------------------------------

/// Encrypts / decrypts media frames for a single RTP sender or receiver.
///
/// Create one per track that should be encrypted. Attach it to the
/// [`RtpSender`] (outbound) or [`RtpReceiver`] (inbound) that corresponds
/// to the participant's media track.
pub struct LkFrameCryptor {
    inner: lk_fc::FrameCryptor,
}

impl LkFrameCryptor {
    pub fn new_for_rtp_sender(
        pcf: &libwebrtc::peer_connection_factory::PeerConnectionFactory,
        participant_id: String,
        algorithm: EncryptionAlgorithm,
        key_provider: &LkKeyProvider,
        sender: RtpSender,
    ) -> Self {
        Self {
            inner: lk_fc::FrameCryptor::new_for_rtp_sender(
                pcf,
                participant_id,
                algorithm,
                key_provider.inner.clone(),
                sender,
            ),
        }
    }

    pub fn new_for_rtp_receiver(
        pcf: &libwebrtc::peer_connection_factory::PeerConnectionFactory,
        participant_id: String,
        algorithm: EncryptionAlgorithm,
        key_provider: &LkKeyProvider,
        receiver: RtpReceiver,
    ) -> Self {
        Self {
            inner: lk_fc::FrameCryptor::new_for_rtp_receiver(
                pcf,
                participant_id,
                algorithm,
                key_provider.inner.clone(),
                receiver,
            ),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.inner.set_enabled(enabled);
    }

    pub fn enabled(&self) -> bool {
        self.inner.enabled()
    }

    pub fn set_key_index(&self, index: i32) {
        self.inner.set_key_index(index);
    }

    pub fn key_index(&self) -> i32 {
        self.inner.key_index()
    }

    pub fn participant_id(&self) -> String {
        self.inner.participant_id()
    }

    pub fn on_state_change(&self, handler: Option<lk_fc::OnStateChange>) {
        self.inner.on_state_change(handler);
    }
}

// ---------------------------------------------------------------------------
// DataPacketCryptor
// ---------------------------------------------------------------------------

/// Encrypts / decrypts data channel packets.
pub struct LkDataPacketCryptor {
    inner: lk_fc::DataPacketCryptor,
}

impl LkDataPacketCryptor {
    pub fn new(algorithm: EncryptionAlgorithm, key_provider: &LkKeyProvider) -> Self {
        Self {
            inner: lk_fc::DataPacketCryptor::new(algorithm, key_provider.inner.clone()),
        }
    }

    pub fn encrypt(
        &self,
        participant_id: &str,
        key_index: u32,
        data: &[u8],
    ) -> Result<EncryptedPacket, Box<dyn std::error::Error>> {
        self.inner.encrypt(participant_id, key_index, data)
    }

    pub fn decrypt(
        &self,
        participant_id: &str,
        encrypted_packet: &EncryptedPacket,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        self.inner.decrypt(participant_id, encrypted_packet)
    }
}

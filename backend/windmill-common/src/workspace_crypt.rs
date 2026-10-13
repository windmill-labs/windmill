/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! The cipher behind `variables::encrypt`/`decrypt`.
//!
//! Values are written as AES-256-GCM ("v2"), standard base64 of
//! `MAGIC (4) || nonce (12) || AES-256-GCM(PKCS#7(plaintext)) || tag (16)`.
//! Values written before are AES-256-CBC (`magic_crypt`) and are still read.
//!
//! The plaintext is padded to the block size so a v2 value decodes to a multiple of 16 bytes,
//! like a CBC one: the CLI and `validate_already_encrypted_secret` tell ciphertext from
//! plaintext by exactly that, and an old CLI that took a v2 value for plaintext would push it
//! to be encrypted again, corrupting the secret.

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::Engine;
use hkdf::Hkdf;
use magic_crypt::{MagicCrypt256, MagicCryptTrait};
use sha2::Sha256;

use crate::min_version::MIN_VERSION_SUPPORTS_AEAD_SECRETS;

pub const V2_MAGIC: [u8; 4] = *b"wmA2";
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const BLOCK: usize = 16;

/// The most `encrypt_bytes` adds to a plaintext, in either format.
pub const MAX_BYTES_OVERHEAD: usize = V2_MAGIC.len() + NONCE_LEN + BLOCK + TAG_LEN;

const AEAD_KEY_INFO: &[u8] = b"windmill workspace secret aes-256-gcm v1";

const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// Why a value did not decrypt. Only ever logged, never returned to a caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecryptFailure {
    Encoding,
    Authentication,
    Utf8,
}

impl std::fmt::Display for DecryptFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            DecryptFailure::Encoding => "not base64",
            DecryptFailure::Authentication => "authentication or padding check failed",
            DecryptFailure::Utf8 => "plaintext is not UTF-8",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherFormat {
    LegacyCbc,
    V2,
}

impl CipherFormat {
    /// What new values are written as. Old workers read secrets straight from the database with
    /// the CBC cipher alone, so v2 is written only once every live worker reads it.
    pub fn current() -> Self {
        if MIN_VERSION_SUPPORTS_AEAD_SECRETS.met_conservatively() {
            CipherFormat::V2
        } else {
            CipherFormat::LegacyCbc
        }
    }
}

#[derive(Clone)]
pub struct WorkspaceCrypt {
    legacy: MagicCrypt256,
    aead: Aes256Gcm,
}

/// Code that drives `magic_crypt` directly on this cipher (app link secrets) keeps the legacy
/// format it always had.
impl std::ops::Deref for WorkspaceCrypt {
    type Target = MagicCrypt256;
    fn deref(&self) -> &MagicCrypt256 {
        &self.legacy
    }
}

impl WorkspaceCrypt {
    /// `crypt_key` is the key material the CBC cipher was always built from (workspace key, salt,
    /// suffix). The AEAD key is derived from it under its own label, so the two ciphers never
    /// share a key.
    pub fn new(crypt_key: &str) -> Self {
        let mut aead_key = [0u8; 32];
        Hkdf::<Sha256>::new(None, crypt_key.as_bytes())
            .expand(AEAD_KEY_INFO, &mut aead_key)
            .expect("32 bytes is a valid HKDF-SHA256 output length");
        Self {
            legacy: magic_crypt::new_magic_crypt!(crypt_key, 256),
            aead: Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&aead_key)),
        }
    }

    pub fn encrypt_str(&self, plaintext: &str) -> String {
        self.encrypt_str_in(plaintext, CipherFormat::current())
    }

    pub fn encrypt_str_in(&self, plaintext: &str, format: CipherFormat) -> String {
        match format {
            CipherFormat::V2 => B64.encode(self.seal(plaintext.as_bytes())),
            CipherFormat::LegacyCbc => self.legacy.encrypt_str_to_base64(plaintext),
        }
    }

    pub fn decrypt_str(&self, ciphertext: &str) -> Result<String, DecryptFailure> {
        let bytes = B64
            .decode(ciphertext)
            .map_err(|_| DecryptFailure::Encoding)?;
        String::from_utf8(self.decrypt_bytes(&bytes)?).map_err(|_| DecryptFailure::Utf8)
    }

    /// Whether `ciphertext` is a v2 value under this key.
    pub fn is_v2(&self, ciphertext: &str) -> bool {
        B64.decode(ciphertext)
            .is_ok_and(|bytes| self.open(&bytes).is_some())
    }

    /// The v2 form of a legacy `ciphertext`, checked to decrypt back to the same plaintext.
    /// `None` for a v2 value and for anything that does not decrypt under this key.
    pub fn upgrade(&self, ciphertext: &str) -> Option<String> {
        if self.is_v2(ciphertext) {
            return None;
        }
        let plaintext = self.decrypt_str(ciphertext).ok()?;
        let upgraded = self.encrypt_str_in(&plaintext, CipherFormat::V2);
        (self.decrypt_str(&upgraded).ok()? == plaintext).then_some(upgraded)
    }

    pub fn encrypt_bytes(&self, plaintext: &[u8]) -> Vec<u8> {
        match CipherFormat::current() {
            CipherFormat::V2 => self.seal(plaintext),
            CipherFormat::LegacyCbc => self.legacy.encrypt_bytes_to_bytes(plaintext),
        }
    }

    pub fn decrypt_bytes(&self, bytes: &[u8]) -> Result<Vec<u8>, DecryptFailure> {
        match self.open(bytes) {
            Some(plaintext) => Ok(plaintext),
            // Also for a value with the magic: a CBC ciphertext starts with it one time in 2^32.
            None => self
                .legacy
                .decrypt_bytes_to_bytes(bytes)
                .map_err(|_| DecryptFailure::Authentication),
        }
    }

    fn seal(&self, plaintext: &[u8]) -> Vec<u8> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let pad = BLOCK - plaintext.len() % BLOCK;
        let mut padded = Vec::with_capacity(plaintext.len() + pad);
        padded.extend_from_slice(plaintext);
        padded.resize(plaintext.len() + pad, pad as u8);
        let sealed = self
            .aead
            .encrypt(&nonce, padded.as_slice())
            .expect("AES-GCM encryption fails only past 64 GiB of plaintext");
        let mut out = Vec::with_capacity(V2_MAGIC.len() + NONCE_LEN + sealed.len());
        out.extend_from_slice(&V2_MAGIC);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&sealed);
        out
    }

    fn open(&self, data: &[u8]) -> Option<Vec<u8>> {
        let body = data.strip_prefix(&V2_MAGIC)?;
        if body.len() < NONCE_LEN + BLOCK + TAG_LEN || (body.len() - NONCE_LEN) % BLOCK != 0 {
            return None;
        }
        let (nonce, sealed) = body.split_at(NONCE_LEN);
        let mut padded = self.aead.decrypt(Nonce::from_slice(nonce), sealed).ok()?;
        let pad = *padded.last()? as usize;
        if pad == 0
            || pad > BLOCK
            || padded[padded.len() - pad..]
                .iter()
                .any(|b| *b as usize != pad)
        {
            return None;
        }
        padded.truncate(padded.len() - pad);
        Some(padded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "workspace-key";

    #[test]
    fn v2_round_trips() {
        let wc = WorkspaceCrypt::new(KEY);
        for plain in [
            "",
            "s",
            "exactly sixteen!",
            "a longer secret value with ünïcödé",
        ] {
            let a = wc.encrypt_str_in(plain, CipherFormat::V2);
            assert_ne!(a, wc.encrypt_str_in(plain, CipherFormat::V2), "nonce reuse");
            assert!(wc.is_v2(&a));
            assert_eq!(wc.decrypt_str(&a).unwrap(), plain);
        }
    }

    #[test]
    fn legacy_cbc_still_decrypts() {
        let wc = WorkspaceCrypt::new(KEY);
        let legacy = magic_crypt::new_magic_crypt!(KEY, 256).encrypt_str_to_base64("old secret");
        assert!(!wc.is_v2(&legacy));
        assert_eq!(wc.decrypt_str(&legacy).unwrap(), "old secret");
        let upgraded = wc.upgrade(&legacy).unwrap();
        assert!(wc.is_v2(&upgraded));
        assert_eq!(wc.decrypt_str(&upgraded).unwrap(), "old secret");
        assert_eq!(wc.upgrade(&upgraded), None);
    }

    #[test]
    fn magic_prefixed_cbc_still_decrypts() {
        // Too rare to find by search, so built: with magic_crypt's zero IV and SHA-256(key) AES
        // key, the first CBC block is AES(P1), so P1 = AES^-1(MAGIC || zeros).
        let mut block = [0u8; 16];
        block[..4].copy_from_slice(&V2_MAGIC);
        let mut plaintext = {
            use aes_gcm::aes::cipher::{BlockDecrypt, KeyInit};
            use sha2::Digest;
            let cipher = aes_gcm::aes::Aes256::new_from_slice(&Sha256::digest(KEY)).unwrap();
            let mut b = aes_gcm::aes::Block::clone_from_slice(&block);
            cipher.decrypt_block(&mut b);
            b.to_vec()
        };
        plaintext.extend_from_slice(b"tail of the secret value");
        let ciphertext = magic_crypt::new_magic_crypt!(KEY, 256).encrypt_bytes_to_bytes(&plaintext);
        assert!(ciphertext.starts_with(&V2_MAGIC));
        assert_eq!(
            WorkspaceCrypt::new(KEY).decrypt_bytes(&ciphertext).unwrap(),
            plaintext
        );
    }

    #[test]
    fn bytes_round_trip_within_the_overhead_in_both_formats() {
        let wc = WorkspaceCrypt::new(KEY);
        for len in 0..64 {
            let plain = vec![b'x'; len];
            for sealed in [wc.seal(&plain), wc.legacy.encrypt_bytes_to_bytes(&plain)] {
                assert!(sealed.len() <= len + MAX_BYTES_OVERHEAD, "len {len}");
                assert_eq!(wc.decrypt_bytes(&sealed).unwrap(), plain);
            }
        }
    }

    #[test]
    fn v2_passes_the_block_multiple_ciphertext_shape_check() {
        let wc = WorkspaceCrypt::new(KEY);
        for len in 0..64 {
            let decoded = B64
                .decode(wc.encrypt_str_in(&"x".repeat(len), CipherFormat::V2))
                .unwrap();
            assert!(!decoded.is_empty() && decoded.len() % 16 == 0, "len {len}");
        }
    }
}

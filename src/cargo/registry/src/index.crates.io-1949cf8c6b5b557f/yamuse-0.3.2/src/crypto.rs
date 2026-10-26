//! `encraw` stream decryption.
//!
//! `/get-file-info` serves lossless audio as an aes-ctr-encrypted stream
//! (`transport: encraw`) with the key handed back in the same response. the iv is
//! 16 zero bytes and the key length selects aes-128 vs aes-256.
//!
//! ctr is a stream cipher, so [`EncrawDecryptor`] can decrypt chunk by chunk
//! during a streaming download — memory stays o(chunk) even for a 300 mb flac.

use aes::{Aes128, Aes256};
use ctr::Ctr128BE;
use ctr::cipher::{KeyIvInit, StreamCipher};

use crate::error::{Error, Result};

/// a stateful `encraw` decryptor.
///
/// feed it consecutive chunks with [`apply`](Self::apply); each call decrypts in
/// place and advances the keystream. chunks must be applied in stream order.
pub struct EncrawDecryptor {
    cipher: Cipher,
}

/// an aes key schedule runs to about a kilobyte, and the two variants differ by
/// 256 bytes; boxing keeps the decryptor itself pointer-sized.
enum Cipher {
    Aes128(Box<Ctr128BE<Aes128>>),
    Aes256(Box<Ctr128BE<Aes256>>),
}

impl EncrawDecryptor {
    /// build a decryptor from the hex key returned by `/get-file-info`.
    ///
    /// # errors
    ///
    /// returns [`Error::InvalidBitrate`] when the key is not valid hex, or is
    /// neither 16 nor 32 bytes long.
    pub fn from_hex_key(key_hex: &str) -> Result<Self> {
        let key = hex::decode(key_hex.trim())
            .map_err(|e| Error::InvalidBitrate(format!("encraw key is not valid hex: {e}")))?;
        let iv = [0u8; 16];
        let cipher = match key.len() {
            16 => Cipher::Aes128(Box::new(Ctr128BE::<Aes128>::new(
                key.as_slice().into(),
                &iv.into(),
            ))),
            32 => Cipher::Aes256(Box::new(Ctr128BE::<Aes256>::new(
                key.as_slice().into(),
                &iv.into(),
            ))),
            other => {
                return Err(Error::InvalidBitrate(format!(
                    "encraw key must be 16 or 32 bytes, got {other}"
                )));
            },
        };
        Ok(Self { cipher })
    }

    /// decrypt one chunk in place, advancing the keystream.
    pub fn apply(&mut self, chunk: &mut [u8]) {
        match &mut self.cipher {
            Cipher::Aes128(c) => c.apply_keystream(chunk),
            Cipher::Aes256(c) => c.apply_keystream(chunk),
        }
    }
}

/// decrypt a whole `encraw` buffer with its hex key.
///
/// for large files prefer [`EncrawDecryptor`] with a streaming download — this
/// holds the entire track in memory.
///
/// # errors
///
/// returns [`Error::InvalidBitrate`] when the key is malformed; see
/// [`EncrawDecryptor::from_hex_key`].
///
/// # examples
///
/// ```
/// use yamuse::crypto::decrypt_encraw;
///
/// // aes-ctr is symmetric: decrypting twice returns the original bytes
/// let key = "00112233445566778899aabbccddeeff";
/// let once = decrypt_encraw(b"flac payload", key)?;
/// let twice = decrypt_encraw(&once, key)?;
/// assert_eq!(twice, b"flac payload");
/// # Ok::<(), yamuse::Error>(())
/// ```
pub fn decrypt_encraw(data: &[u8], key_hex: &str) -> Result<Vec<u8>> {
    let mut out = data.to_vec();
    EncrawDecryptor::from_hex_key(key_hex)?.apply(&mut out);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY_128: &str = "000102030405060708090a0b0c0d0e0f";
    const KEY_256: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

    #[test]
    fn decryption_is_its_own_inverse() {
        for key in [KEY_128, KEY_256] {
            let plain = b"fLaC\x00\x00\x00\x22 some audio payload".to_vec();
            let encrypted = decrypt_encraw(&plain, key).unwrap();
            assert_ne!(encrypted, plain, "keystream must actually alter the bytes");
            assert_eq!(decrypt_encraw(&encrypted, key).unwrap(), plain);
        }
    }

    #[test]
    fn streaming_in_chunks_matches_one_shot() {
        let plain: Vec<u8> = (0..=255u8).cycle().take(4096 + 37).collect();
        let one_shot = decrypt_encraw(&plain, KEY_256).unwrap();

        // odd chunk sizes deliberately straddle the 16-byte aes block boundary
        let mut streamed = Vec::new();
        let mut dec = EncrawDecryptor::from_hex_key(KEY_256).unwrap();
        for chunk in plain.chunks(7) {
            let mut buf = chunk.to_vec();
            dec.apply(&mut buf);
            streamed.extend_from_slice(&buf);
        }
        assert_eq!(streamed, one_shot);
    }

    #[test]
    fn key_length_selects_the_cipher() {
        let plain = b"same input, different key length".to_vec();
        let with_128 = decrypt_encraw(&plain, KEY_128).unwrap();
        let with_256 = decrypt_encraw(&plain, KEY_256).unwrap();
        assert_ne!(with_128, with_256);
    }

    #[test]
    fn malformed_keys_are_rejected() {
        assert!(decrypt_encraw(b"x", "not hex").is_err());
        assert!(
            decrypt_encraw(b"x", "00112233").is_err(),
            "4 bytes is neither 16 nor 32"
        );
        assert!(decrypt_encraw(b"x", "").is_err());
        // 24 bytes: valid hex, valid aes-192 key, but not a length encraw uses
        assert!(decrypt_encraw(b"x", &"ab".repeat(24)).is_err());
    }

    #[test]
    fn keys_tolerate_surrounding_whitespace() {
        assert!(EncrawDecryptor::from_hex_key(&format!("  {KEY_128}\n")).is_ok());
    }

    #[test]
    fn empty_input_stays_empty() {
        assert!(decrypt_encraw(&[], KEY_128).unwrap().is_empty());
    }
}

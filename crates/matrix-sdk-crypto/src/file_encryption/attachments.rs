// Copyright 2020 The Matrix.org Foundation C.I.C.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{
    collections::BTreeMap,
    io::{Error as IoError, ErrorKind, Read},
};

use aes::{
    cipher::{generic_array::GenericArray, FromBlockCipher, NewBlockCipher, StreamCipher},
    Aes256, Aes256Ctr,
};
use base64::DecodeError;
use getrandom::getrandom;
use ruma::{
    events::room::{EncryptedFile, JsonWebKey, JsonWebKeyInit},
    serde::Base64,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use zeroize::Zeroizing;

const IV_SIZE: usize = 16;
const KEY_SIZE: usize = 32;
const VERSION: &str = "v2";

/// A wrapper that transparently encrypts anything that implements `Read` as an
/// Matrix attachment.
pub struct AttachmentDecryptor<'a, R: 'a + Read> {
    inner: &'a mut R,
    expected_hash: Vec<u8>,
    sha: Sha256,
    aes: Aes256Ctr,
}

impl<'a, R: 'a + Read + std::fmt::Debug> std::fmt::Debug for AttachmentDecryptor<'a, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttachmentDecryptor")
            .field("inner", &self.inner)
            .field("expected_hash", &self.expected_hash)
            .finish()
    }
}

impl<'a, R: Read> Read for AttachmentDecryptor<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read_bytes = self.inner.read(buf)?;

        if read_bytes == 0 {
            let hash = self.sha.finalize_reset();

            if hash.as_slice() == self.expected_hash.as_slice() {
                Ok(0)
            } else {
                Err(IoError::new(ErrorKind::Other, "Hash mismatch while decrypting"))
            }
        } else {
            self.sha.update(&buf[0..read_bytes]);
            self.aes.apply_keystream(&mut buf[0..read_bytes]);

            Ok(read_bytes)
        }
    }
}

/// Error type for attachment decryption.
#[derive(Error, Debug)]
pub enum DecryptorError {
    /// Some data in the encrypted attachment coldn't be decoded, this may be a
    /// hash, the secret key, or the initialization vector.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// A hash is missing from the encryption info.
    #[error("The encryption info is missing a hash")]
    MissingHash,
    /// The supplied key or IV has an invalid length.
    #[error("The supplied key or IV has an invalid length.")]
    KeyNonceLength,
    /// The supplied data was encrypted with an unknown version of the
    /// attachment encryption spec.
    #[error("Unknown version for the encrypted attachment.")]
    UnknownVersion,
}

impl<'a, R: Read + 'a> AttachmentDecryptor<'a, R> {
    /// Wrap the given reader decrypting all the data we read from it.
    ///
    /// # Arguments
    ///
    /// * `reader` - The `Reader` that should be wrapped and decrypted.
    ///
    /// * `info` - The encryption info that is necessary to decrypt data from
    /// the reader.
    ///
    /// # Examples
    /// ```
    /// # use std::io::{Cursor, Read};
    /// # use matrix_sdk_crypto::{AttachmentEncryptor, AttachmentDecryptor};
    /// let data = "Hello world".to_owned();
    /// let mut cursor = Cursor::new(data.clone());
    ///
    /// let mut encryptor = AttachmentEncryptor::new(&mut cursor);
    ///
    /// let mut encrypted = Vec::new();
    /// encryptor.read_to_end(&mut encrypted).unwrap();
    /// let info = encryptor.finish();
    ///
    /// let mut cursor = Cursor::new(encrypted);
    /// let mut decryptor = AttachmentDecryptor::new(&mut cursor, info).unwrap();
    /// let mut decrypted_data = Vec::new();
    /// decryptor.read_to_end(&mut decrypted_data).unwrap();
    ///
    /// let decrypted = String::from_utf8(decrypted_data).unwrap();
    /// ```
    pub fn new(
        input: &'a mut R,
        info: MediaEncryptionInfo,
    ) -> Result<AttachmentDecryptor<'a, R>, DecryptorError> {
        if info.version != VERSION {
            return Err(DecryptorError::UnknownVersion);
        }

        let hash =
            info.hashes.get("sha256").ok_or(DecryptorError::MissingHash)?.as_bytes().to_owned();
        let key = Zeroizing::from(info.web_key.k.into_inner());
        let iv = info.iv.into_inner();
        let iv = GenericArray::from_exact_iter(iv).ok_or(DecryptorError::KeyNonceLength)?;

        let sha = Sha256::default();
        let aes = Aes256::new_from_slice(&key).map_err(|_| DecryptorError::KeyNonceLength)?;
        let aes = Aes256Ctr::from_block_cipher(aes, &iv);

        Ok(AttachmentDecryptor { inner: input, expected_hash: hash, sha, aes })
    }
}

/// A wrapper that transparently encrypts anything that implements `Read`.
pub struct AttachmentEncryptor<'a, R: Read + 'a> {
    finished: bool,
    inner: &'a mut R,
    web_key: JsonWebKey,
    iv: Base64,
    hashes: BTreeMap<String, Base64>,
    aes: Aes256Ctr,
    sha: Sha256,
}

impl<'a, R: 'a + Read + std::fmt::Debug> std::fmt::Debug for AttachmentEncryptor<'a, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttachmentEncryptor")
            .field("inner", &self.inner)
            .field("finished", &self.finished)
            .finish()
    }
}

impl<'a, R: Read + 'a> Read for AttachmentEncryptor<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read_bytes = self.inner.read(buf)?;

        if read_bytes == 0 {
            let hash = self.sha.finalize_reset();
            self.hashes
                .entry("sha256".to_owned())
                .or_insert_with(|| Base64::new(hash.as_slice().to_owned()));
            Ok(0)
        } else {
            self.aes.apply_keystream(&mut buf[0..read_bytes]);
            self.sha.update(&buf[0..read_bytes]);

            Ok(read_bytes)
        }
    }
}

impl<'a, R: Read + 'a> AttachmentEncryptor<'a, R> {
    /// Wrap the given reader encrypting all the data we read from it.
    ///
    /// After all the reads are done, and all the data is encrypted that we wish
    /// to encrypt a call to [`finish()`](#method.finish) is necessary to get
    /// the decryption key for the data.
    ///
    /// # Arguments
    ///
    /// * `reader` - The `Reader` that should be wrapped and enrypted.
    ///
    /// # Panics
    ///
    /// Panics if we can't generate enough random data to create a fresh
    /// encryption key.
    ///
    /// # Examples
    /// ```
    /// # use std::io::{Cursor, Read};
    /// # use matrix_sdk_crypto::AttachmentEncryptor;
    /// let data = "Hello world".to_owned();
    /// let mut cursor = Cursor::new(data.clone());
    ///
    /// let mut encryptor = AttachmentEncryptor::new(&mut cursor);
    ///
    /// let mut encrypted = Vec::new();
    /// encryptor.read_to_end(&mut encrypted).unwrap();
    /// let key = encryptor.finish();
    /// ```
    pub fn new(reader: &'a mut R) -> Self {
        let mut key = Zeroizing::new([0u8; KEY_SIZE]);
        let mut iv = Zeroizing::new([0u8; IV_SIZE]);

        getrandom(&mut *key).expect("Can't generate randomness");
        // Only populate the first 8 bytes with randomness, the rest is 0
        // initialized for the counter.
        getrandom(&mut iv[0..8]).expect("Can't generate randomness");

        let web_key = JsonWebKey::from(JsonWebKeyInit {
            kty: "oct".to_owned(),
            key_ops: vec!["encrypt".to_owned(), "decrypt".to_owned()],
            alg: "A256CTR".to_owned(),
            k: Base64::new((*key).to_vec()),
            ext: true,
        });
        let encoded_iv = Base64::new((*iv).to_vec());
        let iv = GenericArray::from_slice(&*iv);
        let key = GenericArray::from_slice(&*key);

        let aes = Aes256::new(key);
        let aes = Aes256Ctr::from_block_cipher(aes, iv);

        AttachmentEncryptor {
            finished: false,
            inner: reader,
            iv: encoded_iv,
            web_key,
            hashes: BTreeMap::new(),
            aes,
            sha: Sha256::default(),
        }
    }

    /// Consume the encryptor and get the encryption key.
    pub fn finish(mut self) -> MediaEncryptionInfo {
        let hash = self.sha.finalize();
        self.hashes
            .entry("sha256".to_owned())
            .or_insert_with(|| Base64::new(hash.as_slice().to_owned()));

        MediaEncryptionInfo {
            version: VERSION.to_string(),
            hashes: self.hashes,
            iv: self.iv,
            web_key: self.web_key,
        }
    }
}

/// Struct holding all the information that is needed to decrypt an encrypted
/// file.
#[derive(Debug, Serialize, Deserialize)]
pub struct MediaEncryptionInfo {
    #[serde(rename = "v")]
    /// The version of the encryption scheme.
    pub version: String,
    /// The web key that was used to encrypt the file.
    pub web_key: JsonWebKey,
    /// The initialization vector that was used to encrypt the file.
    pub iv: Base64,
    /// The hashes that can be used to check the validity of the file.
    pub hashes: BTreeMap<String, Base64>,
}

impl From<EncryptedFile> for MediaEncryptionInfo {
    fn from(file: EncryptedFile) -> Self {
        Self { version: file.v, web_key: file.key, iv: file.iv, hashes: file.hashes }
    }
}

#[cfg(test)]
mod test {
    use std::io::{Cursor, Read};

    use serde_json::json;

    use super::{AttachmentDecryptor, AttachmentEncryptor, MediaEncryptionInfo};

    const EXAMPLE_DATA: &[u8] = &[
        179, 154, 118, 127, 186, 127, 110, 33, 203, 33, 33, 134, 67, 100, 173, 46, 235, 27, 215,
        172, 36, 26, 75, 47, 33, 160,
    ];

    fn example_key() -> MediaEncryptionInfo {
        let info = json!({
            "v": "v2",
            "web_key": {
                "kty": "oct",
                "alg": "A256CTR",
                "ext": true,
                "k": "Voq2nkPme_x8no5-Tjq_laDAdxE6iDbxnlQXxwFPgE4",
                "key_ops": ["encrypt", "decrypt"]
            },
            "iv": "i0DovxYdJEcAAAAAAAAAAA",
            "hashes": {
                "sha256": "ANdt819a8bZl4jKy3Z+jcqtiNICa2y0AW4BBJ/iQRAU"
            }
        });

        serde_json::from_value(info).unwrap()
    }

    #[test]
    fn encrypt_decrypt_cycle() {
        let data = "Hello world".to_owned();
        let mut cursor = Cursor::new(data.clone());

        let mut encryptor = AttachmentEncryptor::new(&mut cursor);

        let mut encrypted = Vec::new();

        encryptor.read_to_end(&mut encrypted).unwrap();
        let key = encryptor.finish();
        assert_ne!(encrypted.as_slice(), data.as_bytes());

        let mut cursor = Cursor::new(encrypted);
        let mut decryptor = AttachmentDecryptor::new(&mut cursor, key).unwrap();
        let mut decrypted_data = Vec::new();

        decryptor.read_to_end(&mut decrypted_data).unwrap();

        let decrypted = String::from_utf8(decrypted_data).unwrap();

        assert_eq!(data, decrypted);
    }

    #[test]
    fn real_decrypt() {
        let mut cursor = Cursor::new(EXAMPLE_DATA.to_vec());
        let key = example_key();

        let mut decryptor = AttachmentDecryptor::new(&mut cursor, key).unwrap();
        let mut decrypted_data = Vec::new();

        decryptor.read_to_end(&mut decrypted_data).unwrap();
        let decrypted = String::from_utf8(decrypted_data).unwrap();

        assert_eq!("It's a secret to everybody", decrypted);
    }

    #[test]
    fn decrypt_invalid_hash() {
        let mut cursor = Cursor::new("fake message");
        let key = example_key();

        let mut decryptor = AttachmentDecryptor::new(&mut cursor, key).unwrap();
        let mut decrypted_data = Vec::new();

        assert!(decryptor.read_to_end(&mut decrypted_data).is_err())
    }
}

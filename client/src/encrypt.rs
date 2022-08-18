use anchor_lang::prelude::Pubkey;
use solana_sdk::{signature::Keypair, signer::Signer};

pub struct SharedKey {
    pub receive_key: Key,
    pub transmit_key: Key,
}

impl SharedKey {
    pub fn new_as_senior(senior: &Keypair, junior: &Pubkey) -> Self {
        Self::new_client(
            &Curve25519PublicKey::from(&senior.pubkey()),
            &Curve25519SecretKey::from(senior),
            &Curve25519PublicKey::from(junior),
        )
    }

    pub fn new_as_junior(junior: &Keypair, senior: &Pubkey) -> Self {
        Self::new_server(
            &Curve25519PublicKey::from(&junior.pubkey()),
            &Curve25519SecretKey::from(junior),
            &Curve25519PublicKey::from(senior),
        )
    }

    pub fn new_client(
        client_public: &Curve25519PublicKey,
        client_secret: &Curve25519SecretKey,
        server_public: &Curve25519PublicKey,
    ) -> Self {
        let mut shared_key = SharedKey {
            receive_key: Key([0; libsodium_sys::crypto_kx_SESSIONKEYBYTES as usize]),
            transmit_key: Key([0; libsodium_sys::crypto_kx_SESSIONKEYBYTES as usize]),
        };
        unsafe {
            assert_eq!(
                libsodium_sys::crypto_kx_client_session_keys(
                    shared_key.receive_key.0.as_mut_ptr(),
                    shared_key.transmit_key.0.as_mut_ptr(),
                    client_public.0.as_ptr(),
                    client_secret.0.as_ptr(),
                    server_public.0.as_ptr()
                ),
                0
            );
        }
        shared_key
    }

    pub fn new_server(
        server_public: &Curve25519PublicKey,
        server_secret: &Curve25519SecretKey,
        client_public: &Curve25519PublicKey,
    ) -> Self {
        let mut shared_key = SharedKey {
            receive_key: Key([0; libsodium_sys::crypto_kx_SESSIONKEYBYTES as usize]),
            transmit_key: Key([0; libsodium_sys::crypto_kx_SESSIONKEYBYTES as usize]),
        };
        unsafe {
            assert_eq!(
                libsodium_sys::crypto_kx_server_session_keys(
                    shared_key.receive_key.0.as_mut_ptr(),
                    shared_key.transmit_key.0.as_mut_ptr(),
                    server_public.0.as_ptr(),
                    server_secret.0.as_ptr(),
                    client_public.0.as_ptr()
                ),
                0
            );
        }
        shared_key
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Key([u8; libsodium_sys::crypto_kx_SESSIONKEYBYTES as usize]);

impl Key {
    pub fn encrypt(&self, plaintext: &str) -> Vec<u8> {
        assert_eq!(
            libsodium_sys::crypto_kx_SESSIONKEYBYTES,
            libsodium_sys::crypto_secretbox_KEYBYTES
        );
        let nonce = [0; libsodium_sys::crypto_secretbox_NONCEBYTES as usize];
        let mut ciphertext = std::iter::repeat(0)
            .take(libsodium_sys::crypto_secretbox_MACBYTES as usize + plaintext.len())
            .collect::<Vec<u8>>();
        unsafe {
            assert_eq!(
                libsodium_sys::crypto_secretbox_easy(
                    ciphertext.as_mut_ptr(),
                    plaintext.as_ptr(),
                    plaintext.len() as u64,
                    nonce.as_ptr(),
                    self.0.as_ptr()
                ),
                0,
                "Encryption successfull"
            );
        }
        ciphertext
    }

    pub fn decrypt(&self, ciphertext: &[u8]) -> String {
        assert_eq!(
            libsodium_sys::crypto_kx_SESSIONKEYBYTES,
            libsodium_sys::crypto_secretbox_KEYBYTES
        );
        let nonce = [0; libsodium_sys::crypto_secretbox_NONCEBYTES as usize];
        let mut plaintext = std::iter::repeat(0)
            .take(ciphertext.len() - libsodium_sys::crypto_secretbox_MACBYTES as usize)
            .collect::<Vec<u8>>();
        unsafe {
            assert_eq!(
                libsodium_sys::crypto_secretbox_open_easy(
                    plaintext.as_mut_ptr(),
                    ciphertext.as_ptr(),
                    ciphertext.len() as u64,
                    nonce.as_ptr(),
                    self.0.as_ptr()
                ),
                0,
                "Decryption successfull"
            );
        }
        String::from_utf8(plaintext).expect("Decrypted plaintext is utf8")
    }
}

pub struct Curve25519SecretKey([u8; libsodium_sys::crypto_scalarmult_curve25519_BYTES as usize]);

impl<'a> From<&'a Keypair> for Curve25519SecretKey {
    fn from(keypair: &'a Keypair) -> Self {
        assert_eq!(
            keypair.to_bytes().len(),
            libsodium_sys::crypto_sign_ed25519_SECRETKEYBYTES as usize
        );
        let mut curve25519_sk =
            Self([0; libsodium_sys::crypto_scalarmult_curve25519_BYTES as usize]);
        unsafe {
            assert_eq!(
                libsodium_sys::crypto_sign_ed25519_sk_to_curve25519(
                    &mut curve25519_sk.0 as *mut u8,
                    &keypair.to_bytes() as *const u8
                ),
                0,
                "Converted signing secret key to encryption secret key"
            );
        }
        curve25519_sk
    }
}

pub struct Curve25519PublicKey([u8; libsodium_sys::crypto_scalarmult_curve25519_BYTES as usize]);

impl<'a> From<&'a Pubkey> for Curve25519PublicKey {
    fn from(pubkey: &'a Pubkey) -> Self {
        assert_eq!(
            pubkey.to_bytes().len(),
            libsodium_sys::crypto_sign_ed25519_PUBLICKEYBYTES as usize
        );
        let mut curve25519_pk =
            Self([0; libsodium_sys::crypto_scalarmult_curve25519_BYTES as usize]);
        unsafe {
            assert_eq!(
                libsodium_sys::crypto_sign_ed25519_pk_to_curve25519(
                    &mut curve25519_pk.0 as *mut u8,
                    &pubkey.to_bytes() as *const u8
                ),
                0,
                "Converted signing public key to encryption public key"
            );
        }
        curve25519_pk
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_encryption() {
        let senior = Keypair::new();
        let junior = Keypair::new();

        let senior_key = SharedKey::new_as_senior(&senior, &junior.pubkey());
        let junior_key = SharedKey::new_as_junior(&junior, &senior.pubkey());

        assert_eq!(senior_key.transmit_key, junior_key.receive_key);
        assert_eq!(junior_key.transmit_key, senior_key.receive_key);

        let plaintext = "Hello world!";
        let ciphertext = senior_key.transmit_key.encrypt(plaintext);
        assert_eq!(junior_key.receive_key.decrypt(&ciphertext), plaintext);
    }
}

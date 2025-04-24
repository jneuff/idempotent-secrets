#[cfg(test)]
mod tests {
    use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};

    #[test]
    fn should_generate_keypair() {
        let mut rng = rand::thread_rng();
        let priv_key = RsaPrivateKey::new(&mut rng, 4096).expect("failed to generate a key");
        let pub_key = RsaPublicKey::from(&priv_key);
        // Encrypt
        let data = b"hello world";
        let enc_data = pub_key
            .encrypt(&mut rng, Pkcs1v15Encrypt, &data[..])
            .expect("failed to encrypt");
        assert_ne!(&data[..], &enc_data[..]);

        // Decrypt
        let dec_data = priv_key
            .decrypt(Pkcs1v15Encrypt, &enc_data)
            .expect("failed to decrypt");
        assert_eq!(&data[..], &dec_data[..]);
    }
}

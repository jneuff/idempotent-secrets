#[cfg(test)]
mod tests {
    use rsa::{
        Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey,
        pkcs1::{EncodeRsaPrivateKey, EncodeRsaPublicKey},
        pkcs8::LineEnding,
    };

    fn generate_keypair() -> Result<(RsaPrivateKey, RsaPublicKey), rsa::Error> {
        let mut rng = rand::thread_rng();
        let priv_key = RsaPrivateKey::new(&mut rng, 4096)?;
        let pub_key = RsaPublicKey::from(&priv_key);
        Ok((priv_key, pub_key))
    }

    fn generate_keypair_pem() -> Result<(String, String), rsa::Error> {
        let (priv_key, pub_key) = generate_keypair()?;
        let priv_pem = priv_key.to_pkcs1_pem(LineEnding::LF)?;
        let pub_pem = pub_key.to_pkcs1_pem(LineEnding::LF)?;
        Ok((priv_pem.to_string(), pub_pem.to_string()))
    }

    #[test]
    fn should_generate_keypair() {
        let mut rng = rand::thread_rng();
        let (priv_key, pub_key) = generate_keypair().unwrap();
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

    #[test]
    fn should_render_pem() {
        let (priv_pem, pub_pem) = generate_keypair_pem().unwrap();
        assert_eq!(
            priv_pem.lines().next().unwrap(),
            "-----BEGIN RSA PRIVATE KEY-----"
        );
        assert_eq!(
            priv_pem.lines().last().unwrap(),
            "-----END RSA PRIVATE KEY-----"
        );
        assert_eq!(
            pub_pem.lines().next().unwrap(),
            "-----BEGIN RSA PUBLIC KEY-----"
        );
        assert_eq!(
            pub_pem.lines().last().unwrap(),
            "-----END RSA PUBLIC KEY-----"
        );
    }
}

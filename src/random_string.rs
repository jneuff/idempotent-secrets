use base64::prelude::*;
use rand::RngCore;

const LENGTH: usize = 128;

pub fn generate_random_string() -> Result<String, rand::Error> {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; LENGTH];
    rng.try_fill_bytes(&mut bytes)?;
    Ok(BASE64_STANDARD.encode(bytes))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_generate_random_string() {
        let random_string = generate_random_string().unwrap();
        dbg!(&random_string);
        assert_eq!(random_string.len(), 172);
    }
}

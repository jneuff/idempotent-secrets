use std::collections::BTreeMap;

mod k8s;
mod keypair;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let (private_key, public_key) = keypair::generate_keypair_pem().unwrap();
    let data = BTreeMap::from([
        (
            "public_key".to_owned(),
            k8s::ByteString(public_key.into_bytes()),
        ),
        (
            "private_key".to_owned(),
            k8s::ByteString(private_key.into_bytes()),
        ),
    ]);
    k8s::create_secret("test-create-secret", "secret1", Some(data))
        .await
        .unwrap();
}

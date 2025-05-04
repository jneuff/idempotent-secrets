use clap::Parser;
use std::collections::BTreeMap;

mod k8s;
mod keypair;

#[derive(Parser)]
struct Args {
    #[clap(long)]
    namespace: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
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
    k8s::create_secret(&args.namespace, "secret1", Some(data))
        .await
        .unwrap();
}

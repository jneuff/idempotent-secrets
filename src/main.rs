use clap::Parser;
use std::collections::BTreeMap;

mod k8s;
mod keypair;
mod random_string;

#[derive(Parser)]
struct Args {
    #[clap(short, long)]
    namespace: String,

    secret_name: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
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
    k8s::create_secret(&args.namespace, &args.secret_name, Some(data))
        .await
        .unwrap();
}

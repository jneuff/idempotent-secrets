use clap::{ArgAction, Parser};
use serde::Deserialize;
use std::collections::BTreeMap;

mod k8s;
mod keypair;
mod random_string;

#[derive(Debug, Parser)]
struct Args {
    #[clap(short, long)]
    namespace: String,

    #[arg(short, long, value_parser = parse_secret, action = ArgAction::Append)]
    json: Vec<Secret>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
enum Secret {
    RsaKeypair { name: String },
}

fn parse_secret(raw: &str) -> Result<Secret, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let secret: Secret = serde_json::from_str(raw)?;
    Ok(secret)
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    for secret in args.json {
        match secret {
            Secret::RsaKeypair { name } => {
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
                k8s::create_secret(&args.namespace, &name, Some(data))
                    .await
                    .unwrap();
            }
        }
    }
}

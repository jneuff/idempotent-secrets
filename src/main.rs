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
    #[arg(short, long)]
    anchor_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
enum Secret {
    RsaKeypair { name: String },
    RandomString { name: String },
}

impl Secret {
    fn name(&self) -> &str {
        match self {
            Secret::RsaKeypair { name } => name,
            Secret::RandomString { name } => name,
        }
    }
}

fn parse_secret(raw: &str) -> Result<Secret, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let secret: Secret = serde_json::from_str(raw)?;
    Ok(secret)
}

async fn handle_secret(
    secret: &Secret,
    namespace: &str,
    owner_reference: Option<&k8s::OwnerReference>,
) -> Result<(), anyhow::Error> {
    match &secret {
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
            k8s::create_secret(namespace, name, Some(data), owner_reference).await?
        }
        Secret::RandomString { name } => {
            let data = BTreeMap::from([(
                "value".to_owned(),
                k8s::ByteString(random_string::generate_random_string()?.into_bytes()),
            )]);
            k8s::create_secret(namespace, name, Some(data), owner_reference).await?
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let owner_reference = match args.anchor_name {
        Some(name) => Some(
            k8s::owner_reference(&args.namespace, &name)
                .await
                .expect("Failed to get owner reference"),
        ),
        None => None,
    };
    for secret in args.json {
        if k8s::get_secret(&args.namespace, secret.name())
            .await
            .is_some()
        {
            println!("Secret {} already exists, skipping", secret.name());
            continue;
        }
        let result = handle_secret(&secret, &args.namespace, owner_reference.as_ref()).await;
        if let Err(e) = result {
            eprintln!("Error creating secret {secret:?}: {e}");
        }
    }
}

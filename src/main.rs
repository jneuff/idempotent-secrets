fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod test {
    use k8s_openapi::api::core::v1::{Namespace, Secret};
    use kube::{
        Api, Client,
        api::{ObjectMeta, PostParams},
    };

    async fn create_namespace(client: &Client, name: &str) -> Result<Namespace, kube::Error> {
        let namespaces: Api<Namespace> = Api::all(client.clone());
        namespaces
            .create(
                &PostParams::default(),
                &Namespace {
                    metadata: ObjectMeta {
                        name: Some(name.to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .await
    }

    fn secret(name: &str) -> Secret {
        Secret {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    async fn create_secret(namespace: &str, name: &str) -> Result<(), kube::Error> {
        let client = Client::try_default().await.unwrap();
        let secrets: Api<Secret> = Api::namespaced(client, namespace);
        let new_secret = secret(name);
        if secrets.get(name).await.is_ok() {
            return Ok(());
        }
        secrets
            .create(&PostParams::default(), &new_secret)
            .await
            .map(|_| ())
    }

    #[tokio::test]
    async fn should_create_secret() {
        let client = Client::try_default().await.unwrap();
        create_namespace(&client, "create").await.unwrap();
        create_secret("create", "create-secret").await.unwrap();
        let secrets: Api<Secret> = Api::namespaced(client, "create");
        let actual_secret = secrets.get("create-secret").await.unwrap();
        assert_eq!(
            actual_secret.metadata.name,
            Some("create-secret".to_string())
        );
    }

    #[tokio::test]
    async fn should_not_create_secret_if_exists() {
        let client = Client::try_default().await.unwrap();
        create_namespace(&client, "idempotent").await.unwrap();
        create_secret("idempotent", "idempotent-secret").await.unwrap();
        create_secret("idempotent", "idempotent-secret").await.unwrap();
    }
}

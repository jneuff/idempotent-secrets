fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod test {
    use std::process::Command;

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

    #[tokio::test]
    async fn should_create_kwok_cluster() {
        Command::new("kwokctl")
            .args(["create", "cluster", "--name", "should_create_kwok_cluster"])
            .output()
            .unwrap();
        let client = Client::try_default().await.unwrap();
        create_namespace(&client, "new").await.unwrap();
        let secrets: Api<Secret> = Api::namespaced(client, "new");
        let new_secret = secret("new-secret");
        secrets
            .create(&PostParams::default(), &new_secret)
            .await
            .unwrap();
        let actual_secret = secrets.get("new-secret").await.unwrap();
        assert_eq!(actual_secret.metadata.name, Some("new-secret".to_string()));
        Command::new("kwokctl")
            .args(["delete", "cluster", "--name", "should_create_kwok_cluster"])
            .output()
            .unwrap();
    }
}

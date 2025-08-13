use std::collections::BTreeMap;

pub use k8s_openapi::ByteString;
use k8s_openapi::api::core::v1::Secret;
use kube::{
    Api, Client,
    api::{ObjectMeta, PostParams},
};

fn secret(name: &str, data: Option<BTreeMap<String, ByteString>>) -> Secret {
    Secret {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            ..Default::default()
        },
        data,
        ..Default::default()
    }
}

pub async fn get_secret(namespace: &str, name: &str) -> Option<Secret> {
    let client = Client::try_default().await.unwrap();
    let secrets: Api<Secret> = Api::namespaced(client, namespace);
    secrets.get(name).await.ok()
}

pub async fn create_secret(
    namespace: &str,
    name: &str,
    data: Option<BTreeMap<String, ByteString>>,
) -> Result<(), kube::Error> {
    let client = Client::try_default().await.unwrap();
    let secrets: Api<Secret> = Api::namespaced(client, namespace);
    let new_secret = secret(name, data);
    secrets
        .create(&PostParams::default(), &new_secret)
        .await
        .map(|_| ())
}

#[cfg(test)]
mod test {
    use super::*;
    use k8s_openapi::api::core::v1::Namespace;

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

    fn any_secret_data() -> BTreeMap<String, ByteString> {
        BTreeMap::from([("foo".to_string(), ByteString("bar".into()))])
    }

    #[tokio::test]
    async fn should_create_and_get_secret() {
        let client = Client::try_default().await.unwrap();
        create_namespace(&client, "test-1").await.unwrap();
        let expected = any_secret_data();

        create_secret("test-1", "secret-1", Some(expected.clone()))
            .await
            .unwrap();

        let actual = get_secret("test-1", "secret-1")
            .await
            .unwrap()
            .data
            .unwrap();

        assert_eq!(actual, expected)
    }
}

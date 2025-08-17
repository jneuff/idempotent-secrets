use std::collections::BTreeMap;

pub use k8s_openapi::ByteString;
use k8s_openapi::{api::core::v1::Secret, apimachinery::pkg::apis::meta::v1::OwnerReference};
use kube::{
    Api, Client,
    api::{ObjectMeta, PostParams},
};

fn secret(
    name: &str,
    data: Option<BTreeMap<String, ByteString>>,
    owner_reference: Option<OwnerReference>,
) -> Secret {
    Secret {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            owner_references: owner_reference.map(|owner| vec![owner]),
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
    owner_reference: Option<OwnerReference>,
) -> Result<(), kube::Error> {
    let client = Client::try_default().await.unwrap();
    let secrets: Api<Secret> = Api::namespaced(client, namespace);
    let new_secret = secret(name, data, owner_reference);
    secrets
        .create(&PostParams::default(), &new_secret)
        .await
        .map(|_| ())
}

#[cfg(test)]
mod test {
    use super::*;
    use k8s_openapi::api::core::v1::{ConfigMap, Namespace};

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

        create_secret("test-1", "secret-1", Some(expected.clone()), None)
            .await
            .unwrap();

        let actual = get_secret("test-1", "secret-1")
            .await
            .unwrap()
            .data
            .unwrap();

        assert_eq!(actual, expected)
    }

    async fn create_configmap(
        client: &Client,
        namespace: &str,
        name: &str,
    ) -> Result<ConfigMap, kube::Error> {
        let configmaps: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
        configmaps
            .create(
                &PostParams::default(),
                &ConfigMap {
                    metadata: ObjectMeta {
                        name: Some(name.to_string()),
                        ..Default::default()
                    },
                    data: None,
                    ..Default::default()
                },
            )
            .await
    }

    #[tokio::test]
    async fn sets_owner_reference() {
        let client = Client::try_default().await.unwrap();
        let namespace = "test-5";
        create_namespace(&client, namespace).await.unwrap();
        let config_map = create_configmap(&client, namespace, "idempotent-secrets")
            .await
            .unwrap();
        let owner_reference = OwnerReference {
            api_version: "v1".to_string(),
            kind: "ConfigMap".to_string(),
            name: config_map.metadata.name.unwrap(),
            uid: config_map.metadata.uid.unwrap(),
            ..Default::default()
        };

        create_secret(
            namespace,
            "secret-2",
            Some(any_secret_data()),
            Some(owner_reference),
        )
        .await
        .unwrap();

        let owner_references = get_secret(namespace, "secret-2")
            .await
            .unwrap()
            .metadata
            .owner_references
            .unwrap();

        assert_eq!(owner_references.len(), 1);
        assert_eq!(owner_references[0].name, "idempotent-secrets");
        assert_eq!(owner_references[0].kind, "ConfigMap");
    }
}

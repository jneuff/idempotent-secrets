use std::collections::BTreeMap;

pub use k8s_openapi::ByteString;
use k8s_openapi::api::core::v1::Secret;
use kube::{
    Api, Client,
    api::{DeleteParams, ListParams, ObjectMeta, PostParams},
};

pub use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;

pub async fn delete_secret(namespace: &str, name: &str) -> Result<(), kube::Error> {
    let client = Client::try_default().await.unwrap();
    let secrets: Api<Secret> = Api::namespaced(client, namespace);
    secrets
        .delete(name, &DeleteParams::default())
        .await
        .map(|_| ())
}

pub async fn list_owned_secrets(namespace: &str, owner: &str) -> Result<Vec<String>, kube::Error> {
    let client = Client::try_default().await.unwrap();
    let secrets: Api<Secret> = Api::namespaced(client, namespace);
    let label_selector = format!("owner={owner}");
    Ok(secrets
        .list_metadata(&ListParams::default().labels(&label_selector))
        .await?
        .into_iter()
        .filter_map(|s| s.metadata.name)
        .collect())
}
fn secret(
    name: &str,
    data: Option<BTreeMap<String, ByteString>>,
    owner_reference: Option<&OwnerReference>,
    labels: Option<BTreeMap<String, String>>,
) -> Secret {
    Secret {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            owner_references: owner_reference.map(|owner| vec![owner.clone()]),
            labels,
            ..Default::default()
        },
        data,
        ..Default::default()
    }
}

pub fn labels_from_owner_reference(owner_reference: &OwnerReference) -> BTreeMap<String, String> {
    [("owner".to_string(), owner_reference.name.clone())].into()
}

pub async fn owner_reference(
    namespace: &str,
    anchor_name: &str,
) -> Result<OwnerReference, anyhow::Error> {
    let secret = get_secret(namespace, anchor_name)
        .await
        .ok_or(anyhow::anyhow!("Anchor secret {} not found", anchor_name))?;
    Ok(OwnerReference {
        api_version: "v1".to_string(),
        kind: "Secret".to_string(),
        name: secret.metadata.name.clone().unwrap(),
        uid: secret.metadata.uid.clone().unwrap(),
        ..Default::default()
    })
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
    owner_reference: Option<&OwnerReference>,
    labels: Option<BTreeMap<String, String>>,
) -> Result<Secret, kube::Error> {
    let client = Client::try_default().await.unwrap();
    let secrets: Api<Secret> = Api::namespaced(client, namespace);
    let new_secret = secret(name, data, owner_reference, labels);
    secrets.create(&PostParams::default(), &new_secret).await
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

        create_secret("test-1", "secret-1", Some(expected.clone()), None, None)
            .await
            .unwrap();

        let actual = get_secret("test-1", "secret-1")
            .await
            .unwrap()
            .data
            .unwrap();

        assert_eq!(actual, expected)
    }

    #[tokio::test]
    async fn sets_owner_reference() {
        let client = Client::try_default().await.unwrap();
        let namespace = "test-sets-owner-reference";
        create_namespace(&client, namespace).await.unwrap();
        let secret = create_secret(namespace, "idempotent-secrets", None, None, None)
            .await
            .unwrap();
        let owner_reference = OwnerReference {
            api_version: "v1".to_string(),
            kind: "Secret".to_string(),
            name: secret.metadata.name.unwrap(),
            uid: secret.metadata.uid.unwrap(),
            ..Default::default()
        };

        create_secret(
            namespace,
            "secret-2",
            Some(any_secret_data()),
            Some(&owner_reference),
            None,
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
        assert_eq!(owner_references[0].kind, "Secret");
    }

    #[tokio::test]
    async fn lists_only_owned_secrets() {
        let client = Client::try_default().await.unwrap();
        let namespace = "test-list";
        create_namespace(&client, namespace).await.unwrap();
        create_secret(
            namespace,
            "secret-1",
            None,
            None,
            Some([("owner".to_string(), "foo".to_string())].into()),
        )
        .await
        .unwrap();
        create_secret(namespace, "secret-2", None, None, None)
            .await
            .unwrap();

        let mut secrets = list_owned_secrets(namespace, "foo").await.unwrap();
        secrets.sort();

        assert_eq!(secrets, ["secret-1"])
    }

    #[tokio::test]
    async fn deletes_secret() {
        let client = Client::try_default().await.unwrap();
        let namespace = "test-delete-3";
        create_namespace(&client, namespace).await.unwrap();
        create_secret(namespace, "secret-1", None, None, None)
            .await
            .unwrap();

        delete_secret(namespace, "secret-1").await.unwrap();

        let result = get_secret(namespace, "secret-1").await;
        assert!(result.is_none())
    }
}

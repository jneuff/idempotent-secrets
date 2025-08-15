use lazy_static::lazy_static;
use std::process::{Command, ExitStatus};

mod helpers;

use helpers::*;
use serde_json::{Value, json};

const HELM_COMMAND_TIMEOUT: &str = "60s";

lazy_static! {
    static ref IMAGE_TAG: String = initialize_image_tag();
}

fn image_tag_from_values() -> &'static str {
    let values = include_str!("../helm/idempotent-secrets/values.yaml");
    values
        .lines()
        .find(|line| line.contains("tag:"))
        .and_then(|line| line.split("tag: ").last())
        .expect("tag must be set")
}

fn image_tag() -> &'static str {
    match option_env!("GITHUB_IMAGE_TAG") {
        Some("") => image_tag_from_values(),
        Some(sha) => sha,
        None => "local",
    }
}

fn load_image() {
    if std::env::var("GITHUB_CI").is_err() {
        // Load docker image
        let status = Command::new("kind")
            .args([
                "load",
                "docker-image",
                &format!("idempotent-secrets:{}", image_tag()),
                "--name",
                CLUSTER.name(),
            ])
            .status()
            .expect("Failed to execute kind load docker-image command");
        assert!(status.success(), "Failed to load docker image");
    }
}

fn initialize_image_tag() -> String {
    load_image();
    format!(r#"image.tag={}"#, image_tag())
}

pub struct HelmUpgrade {
    namespace: String,
    release_name: String,
    secrets: Vec<Value>,
    values: Vec<(String, String)>,
}

impl HelmUpgrade {
    fn in_namespace(namespace: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            release_name: "idempotent-secrets".to_string(),
            secrets: vec![],
            values: vec![],
        }
    }

    fn with_secret(mut self, secret: Value) -> Self {
        self.secrets.push(secret);
        self
    }

    fn release_name(self, release_name: &str) -> Self {
        Self {
            release_name: release_name.to_string(),
            ..self
        }
    }

    fn set(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.values.push((key.into(), value.into()));
        self
    }

    fn secrets(&self) -> String {
        format!("secrets={}", serde_json::to_string(&self.secrets).unwrap())
    }

    fn run(self) -> Result<ExitStatus, anyhow::Error> {
        let secrets = self.secrets();
        let mut args = vec![
            "upgrade",
            "--install",
            &self.release_name,
            "./helm/idempotent-secrets",
            "--namespace",
            &self.namespace,
            "--set",
            &IMAGE_TAG,
            "--set-json",
            &secrets,
            "--wait",
            "--wait-for-jobs",
            "--timeout",
            HELM_COMMAND_TIMEOUT,
        ];
        if std::env::var("GITHUB_CI").is_err() {
            args.extend(["--set", r#"image.repository="#]);
        }
        let rendered_values = self
            .values
            .iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect::<Vec<_>>();
        for value in rendered_values.iter() {
            args.extend(["--set", value]);
        }
        Command::new("helm")
            .args(&args)
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to execute helm upgrade command: {}", e))
    }
}

#[test]
fn create_rsa_keypair_secret() {
    let namespace = given_a_namespace!();
    let secret_name = "rsa-key";

    let status = HelmUpgrade::in_namespace(namespace.name())
        .with_secret(json!({
            "name": secret_name,
            "type": "RsaKeypair",
        }))
        .run()
        .unwrap();
    assert!(status.success(), "Failed to install Helm chart");

    let secret = kubectl_get_secret(namespace.name(), secret_name).unwrap();
    assert_eq!(secret["metadata"]["name"].as_str().unwrap(), secret_name);
}

#[test]
fn create_random_string_secret() {
    let namespace = given_a_namespace!();
    let secret_name = "secret-1";

    let status = HelmUpgrade::in_namespace(namespace.name())
        .with_secret(json!({
            "name": secret_name,
            "type": "RandomString",
        }))
        .run()
        .unwrap();
    assert!(status.success(), "Failed to install Helm chart");

    // Verify secret creation
    let secret = kubectl_get_secret(namespace.name(), secret_name).unwrap();
    assert_eq!(secret["metadata"]["name"].as_str().unwrap(), secret_name);
}

#[test]
fn configure_multiple_secrets() {
    let namespace = given_a_namespace!();
    let secret_names = &["secret-1", "secret-2"];

    let status = HelmUpgrade::in_namespace(namespace.name())
        .with_secret(json!({
            "name": secret_names[0],
            "type": "RsaKeypair",
        }))
        .with_secret(json!({
            "name": secret_names[1],
            "type": "RandomString",
        }))
        .run()
        .unwrap();
    assert!(status.success(), "Failed to install Helm chart");
    // Verify secret creation
    for secret_name in secret_names {
        let secret = kubectl_get_secret(namespace.name(), secret_name).unwrap();
        assert_eq!(secret["metadata"]["name"].as_str().unwrap(), *secret_name);
    }
}

fn enforce_pod_security_standards(namespace: &str) -> Result<(), anyhow::Error> {
    Command::new("kubectl")
        .args([
            "label",
            "namespace",
            namespace,
            "pod-security.kubernetes.io/enforce=restricted",
        ])
        .status()?;
    Ok(())
}

#[test]
fn should_adhere_to_pod_security_standards() {
    let namespace = given_a_namespace!();
    let secret_name = "rsa-key";
    enforce_pod_security_standards(namespace.name()).unwrap();

    let status = HelmUpgrade::in_namespace(namespace.name())
        .with_secret(json!({
            "name": secret_name,
            "type": "RsaKeypair",
        }))
        .run()
        .unwrap();
    assert!(status.success(), "Failed to install Helm chart");
    let secret = kubectl_get_secret(namespace.name(), secret_name).unwrap();
    assert_eq!(secret["metadata"]["name"].as_str().unwrap(), secret_name);
}

#[test]
fn install_two_releases_with_different_names() {
    let namespace = given_a_namespace!();
    let secret_name = "rsa-key";

    let status = HelmUpgrade::in_namespace(namespace.name())
        .release_name("idempotent-secrets-1")
        .with_secret(json!({
            "name": secret_name,
            "type": "RsaKeypair",
        }))
        .run()
        .unwrap();
    assert!(status.success(), "Failed to install Helm chart");
    let secret = kubectl_get_secret(namespace.name(), secret_name).unwrap();
    assert_eq!(secret["metadata"]["name"].as_str().unwrap(), secret_name);

    let secret2_name = "rsa-key-2";
    let status = HelmUpgrade::in_namespace(namespace.name())
        .release_name("idempotent-secrets-2")
        .with_secret(json!({
            "name": secret2_name,
            "type": "RsaKeypair",
        }))
        .run()
        .unwrap();
    assert!(
        status.success(),
        "Failed to install Helm chart a second time"
    );
    let secret = kubectl_get_secret(namespace.name(), secret2_name).unwrap();
    assert_eq!(secret["metadata"]["name"].as_str().unwrap(), secret2_name);
}

#[test]
fn supports_fullname_override() {
    let namespace = given_a_namespace!();

    let status = HelmUpgrade::in_namespace(namespace.name())
        .set("fullnameOverride", "custom-name")
        .with_secret(json!({
            "name": "rsa-key",
            "type": "RsaKeypair",
        }))
        .run()
        .unwrap();
    assert!(status.success(), "Failed to install Helm chart");

    let output = Command::new("kubectl")
        .args(["get", "pod", "-n", namespace.name()])
        .output()
        .expect("Failed to execute kubectl get pod command");

    assert!(output.status.success(), "Failed to get pod");
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        output_str.contains("custom-name"),
        "Pod name does not contain custom-name"
    );

    let output = Command::new("kubectl")
        .args(["get", "serviceaccount", "-n", namespace.name()])
        .output()
        .expect("Failed to execute kubectl get serviceaccount command");

    assert!(output.status.success(), "Failed to get serviceaccount");
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        output_str.contains("custom-name"),
        "Serviceaccount name does not contain custom-name"
    );

    let output = Command::new("kubectl")
        .args(["get", "role", "-n", namespace.name()])
        .output()
        .expect("Failed to execute kubectl get role command");

    assert!(output.status.success(), "Failed to get role");
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        output_str.contains("custom-name"),
        "Role name does not contain custom-name"
    );

    let output = Command::new("kubectl")
        .args(["get", "rolebinding", "-n", namespace.name()])
        .output()
        .expect("Failed to execute kubectl get rolebinding command");

    assert!(output.status.success(), "Failed to get role");
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        output_str.contains("custom-name"),
        "Role name does not contain custom-name"
    );
}

fn helm_uninstall(namespace: &str, release_name: &str) -> Result<ExitStatus, anyhow::Error> {
    Command::new("helm")
        .args([
            "uninstall",
            release_name,
            "--namespace",
            namespace,
            "--wait",
            "--timeout",
            HELM_COMMAND_TIMEOUT,
        ])
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to execute helm uninstall command: {}", e))
}

#[should_panic]
#[test]
fn deletes_secrets_when_uninstalled() {
    let namespace = given_a_namespace!();
    let secret_name = "rsa-key";
    let release_name = "idempotent-secrets";

    let status = HelmUpgrade::in_namespace(namespace.name())
        .release_name(release_name)
        .with_secret(json!({
            "name": secret_name,
            "type": "RsaKeypair",
        }))
        .run()
        .unwrap();
    assert!(status.success(), "Failed to install Helm chart");

    let secret = kubectl_get_secret(namespace.name(), secret_name).unwrap();
    assert_eq!(secret["metadata"]["name"].as_str().unwrap(), secret_name);

    let status = helm_uninstall(namespace.name(), release_name).unwrap();
    assert!(status.success(), "Failed to uninstall Helm chart");

    let result = kubectl_get_secret(namespace.name(), secret_name);
    assert!(result.is_err(), "Secret should not exist");
}

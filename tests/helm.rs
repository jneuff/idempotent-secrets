use std::process::{Command, ExitStatus};

mod helpers;

use helpers::*;
use serde_json::{Value, json};

const HELM_COMMAND_TIMEOUT: &str = "60s";

pub fn set_image_tag() -> String {
    format!(r#"image.tag={}"#, image_tag())
}

pub struct HelmUpgrade {
    namespace: String,
    release_name: String,
    image_tag: String,
    secrets: Vec<Value>,
}

impl HelmUpgrade {
    pub fn in_namespace(namespace: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            release_name: "idempotent-secrets".to_string(),
            image_tag: set_image_tag(),
            secrets: vec![],
        }
    }

    pub fn with_secret(mut self, secret: Value) -> Self {
        self.secrets.push(secret);
        self
    }

    pub fn release_name(self, release_name: &str) -> Self {
        Self {
            release_name: release_name.to_string(),
            ..self
        }
    }

    fn secrets(&self) -> String {
        format!("secrets={}", serde_json::to_string(&self.secrets).unwrap())
    }

    pub fn run(self) -> Result<ExitStatus, anyhow::Error> {
        let secrets = self.secrets();
        let mut args = vec![
            "upgrade",
            "--install",
            &self.release_name,
            "./helm/idempotent-secrets",
            "--namespace",
            &self.namespace,
            "--set",
            &self.image_tag,
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
        Command::new("helm")
            .args(&args)
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to execute helm upgrade command: {}", e))
    }
}
#[test]
fn test_helm_installation_and_secret_creation() {
    let namespace = given_a_namespace!();
    let secret_name = "rsa-key";
    let set_image_tag = set_image_tag();

    let mut args = vec![
        "upgrade",
        "--install",
        "idempotent-secrets",
        "./helm/idempotent-secrets",
        "--namespace",
        &namespace.name(),
        "--set",
        &set_image_tag,
        "--set-json",
        r#"secrets=[{"name":"rsa-key", "type":"RsaKeypair"}]"#,
        "--wait",
        "--wait-for-jobs",
        "--timeout",
        HELM_COMMAND_TIMEOUT,
    ];
    if std::env::var("GITHUB_CI").is_err() {
        args.extend(["--set", r#"image.repository="#]);
    }
    // Install Helm chart
    let status = Command::new("helm")
        .args(args)
        .status()
        .expect("Failed to execute helm upgrade command");

    assert!(status.success(), "Failed to install Helm chart");

    // Verify secret creation
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
fn allow_multiple_secrets() {
    let namespace = given_a_namespace!();
    let secret_names = &["secret-1", "secret-2"];
    let set_image_tag = set_image_tag();

    let mut args = vec![
        "upgrade",
        "--install",
        "idempotent-secrets",
        "./helm/idempotent-secrets",
        "--namespace",
        &namespace.name(),
        "--set",
        &set_image_tag,
        "--set-json",
        r#"secrets=[{"name":"secret-1", "type":"RsaKeypair"},{"name":"secret-2", "type":"RsaKeypair"}]"#,
        "--wait",
        "--wait-for-jobs",
        "--timeout",
        HELM_COMMAND_TIMEOUT,
    ];
    if std::env::var("GITHUB_CI").is_err() {
        args.extend(["--set", r#"image.repository="#]);
    }
    // Install Helm chart
    let status = Command::new("helm")
        .args(args)
        .status()
        .expect("Failed to execute helm upgrade command");

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
    let set_image_tag = set_image_tag();
    enforce_pod_security_standards(namespace.name()).unwrap();

    let mut args = vec![
        "install",
        "idempotent-secrets",
        "./helm/idempotent-secrets",
        "--namespace",
        &namespace.name(),
        "--set",
        &set_image_tag,
        "--set-json",
        r#"secrets=[{"name":"rsa-key", "type":"RsaKeypair"}]"#,
        "--wait",
        "--wait-for-jobs",
        "--timeout",
        HELM_COMMAND_TIMEOUT,
    ];
    if std::env::var("GITHUB_CI").is_err() {
        args.extend(["--set", r#"image.repository="#]);
    }
    // Install Helm chart
    let status = Command::new("helm")
        .args(args)
        .status()
        .expect("Failed to execute helm install command");

    assert!(status.success(), "Failed to install Helm chart");
    let secret = kubectl_get_secret(namespace.name(), secret_name).unwrap();
    assert_eq!(secret["metadata"]["name"].as_str().unwrap(), secret_name);
}

#[test]
fn should_install_two_releases_with_different_names() {
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
fn should_allow_fullname_override() {
    let namespace = given_a_namespace!();
    let set_image_tag = set_image_tag();

    let mut args = vec![
        "install",
        "idempotent-secrets",
        "./helm/idempotent-secrets",
        "--namespace",
        &namespace.name(),
        "--set",
        &set_image_tag,
        "--set-json",
        r#"secrets=[{"name":"rsa-key", "type":"RsaKeypair"}]"#,
        "--set",
        r#"fullnameOverride="custom-name""#,
        "--wait",
        "--wait-for-jobs",
        "--timeout",
        HELM_COMMAND_TIMEOUT,
    ];
    if std::env::var("GITHUB_CI").is_err() {
        args.extend(["--set", r#"image.repository="#]);
    }
    // Install Helm chart
    let status = Command::new("helm")
        .args(args)
        .status()
        .expect("Failed to execute helm install command");

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

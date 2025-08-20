mod helpers;
use std::process::{Command, Output};

use helpers::*;
use serde::Serialize;
use serde_json::{Value, json};

struct IdempotentSecrets {
    secrets: Vec<String>,
    namespace: String,
    anchor_name: Option<String>,
}

impl IdempotentSecrets {
    fn in_namespace(namespace: &str) -> Self {
        Self {
            secrets: vec![],
            namespace: namespace.to_string(),
            anchor_name: None,
        }
    }

    fn with_secret(mut self, secret: impl Serialize) -> Self {
        self.secrets.push(serde_json::to_string(&secret).unwrap());
        self
    }

    fn set_anchor(mut self, anchor_name: impl Into<String>) -> Self {
        self.anchor_name = Some(anchor_name.into());
        self
    }

    fn run(self) -> Result<Output, std::io::Error> {
        dbg!(
            "running in namespace {} in cluster {}",
            &self.namespace,
            CLUSTER.name()
        );
        let mut args = vec!["run", "--", "--namespace", &self.namespace];
        for secret in &self.secrets {
            args.push("--json");
            args.push(secret);
        }
        if let Some(ref anchor_name) = self.anchor_name {
            args.push("--anchor-name");
            args.push(anchor_name);
        }
        Command::new("cargo").args(&args).output()
    }
}

fn assert_no_errors(output: Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        !stderr.contains("Error creating secret"),
        "stdout: {stdout}\nstderr: {stderr}"
    );
}

#[test]
fn is_idempotent() {
    let namespace = given_a_namespace!();

    let output = IdempotentSecrets::in_namespace(namespace.name())
        .with_secret(json!({ "name": "secret-1", "type": "RandomString" }))
        .run()
        .unwrap();

    assert_no_errors(output);
    let secret_1 = kubectl_get_secret(namespace.name(), "secret-1").unwrap();

    let output = IdempotentSecrets::in_namespace(namespace.name())
        .with_secret(json!({ "name": "secret-1", "type": "RandomString" }))
        .run()
        .unwrap();

    assert_no_errors(output);
    let secret_2 = kubectl_get_secret(namespace.name(), "secret-1").unwrap();
    assert_eq!(secret_1, secret_2);
}

fn kubectl_create_secret(namespace: &str, name: &str) -> Result<Value, anyhow::Error> {
    Command::new("kubectl")
        .args([
            "create",
            "secret",
            "generic",
            name,
            "--namespace",
            namespace,
        ])
        .status()
        .unwrap();

    let output = Command::new("kubectl")
        .args(["get", "secret", name, "-n", namespace, "-ojson"])
        .output()
        .expect("Failed to execute kubectl get secret command");

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "{}\nstdout: {}\nstderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

#[test]
fn sets_owner_reference() {
    let namespace = given_a_namespace!();
    let anchor_secret =
        kubectl_create_secret(namespace.name(), "idempotent-secrets-anchor").unwrap();

    let output = IdempotentSecrets::in_namespace(namespace.name())
        .with_secret(json!({ "name": "secret-1", "type": "RandomString" }))
        .set_anchor(anchor_secret["metadata"]["name"].as_str().unwrap())
        .run()
        .unwrap();
    assert_no_errors(output);

    let secret = kubectl_get_secret(namespace.name(), "secret-1").unwrap();
    let owner_references = secret["metadata"]["ownerReferences"].as_array().unwrap();

    assert_eq!(owner_references.len(), 1);
    assert_eq!(
        owner_references[0]["name"].as_str().unwrap(),
        "idempotent-secrets-anchor"
    );
    assert_eq!(owner_references[0]["kind"].as_str().unwrap(), "Secret");
}

#[should_panic]
#[test]
fn deletes_secrets_when_config_changes() {
    let namespace = given_a_namespace!();

    let output = IdempotentSecrets::in_namespace(namespace.name())
        .with_secret(json!({ "name": "secret-1", "type": "RandomString" }))
        .with_secret(json!({ "name": "secret-2", "type": "RandomString" }))
        .run()
        .unwrap();
    assert_no_errors(output);
    let secret_1 = kubectl_get_secret(namespace.name(), "secret-1").unwrap();
    assert_eq!(secret_1["metadata"]["name"].as_str().unwrap(), "secret-1");
    let secret_2 = kubectl_get_secret(namespace.name(), "secret-2").unwrap();
    assert_eq!(secret_2["metadata"]["name"].as_str().unwrap(), "secret-2");

    let output = IdempotentSecrets::in_namespace(namespace.name())
        .with_secret(json!({ "name": "secret-2", "type": "RandomString" }))
        .run()
        .unwrap();
    assert_no_errors(output);

    let secret_2 = kubectl_get_secret(namespace.name(), "secret-2").unwrap();
    assert_eq!(secret_2["metadata"]["name"].as_str().unwrap(), "secret-2");
    assert!(kubectl_get_secret(namespace.name(), "secret-1").is_err());
}

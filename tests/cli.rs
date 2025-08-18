mod helpers;
use std::process::{Command, Output};

use helpers::*;
use serde::Serialize;
use serde_json::{Value, json};

struct IdempotentSecrets {
    secrets: Vec<String>,
    namespace: String,
    owner_reference: Option<String>,
}

impl IdempotentSecrets {
    fn in_namespace(namespace: &str) -> Self {
        Self {
            secrets: vec![],
            namespace: namespace.to_string(),
            owner_reference: None,
        }
    }

    fn with_secret(mut self, secret: impl Serialize) -> Self {
        self.secrets.push(serde_json::to_string(&secret).unwrap());
        self
    }

    fn set_owner(mut self, owner_reference: impl Serialize) -> Self {
        self.owner_reference = Some(serde_json::to_string(&owner_reference).unwrap());
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
        if let Some(ref owner_reference) = self.owner_reference {
            args.push("--owner-reference");
            args.push(owner_reference);
        }
        Command::new("cargo").args(&args).output()
    }
}

fn assert_no_errors(output: Output) {
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Error creating secret"),
        "stderr: {}",
        stderr
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

fn kubectl_create_config_map(namespace: &str, name: &str) -> Result<Value, anyhow::Error> {
    Command::new("kubectl")
        .args(["create", "configmap", name, "--namespace", namespace])
        .status()
        .unwrap();

    let output = Command::new("kubectl")
        .args(["get", "configmap", name, "-n", namespace, "-ojson"])
        .output()
        .expect("Failed to execute kubectl get configmap command");

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
    let config_map = kubectl_create_config_map(namespace.name(), "idempotent-secrets").unwrap();

    let output = IdempotentSecrets::in_namespace(namespace.name())
        .with_secret(json!({ "name": "secret-1", "type": "RandomString" }))
        .set_owner(json!({
            "api_version": "v1",
            "kind": "ConfigMap",
            "name": config_map["metadata"]["name"].as_str().unwrap(),
            "uid": config_map["metadata"]["uid"].as_str().unwrap(),
        }))
        .run()
        .unwrap();
    assert_no_errors(output);

    let secret = kubectl_get_secret(namespace.name(), "secret-1").unwrap();
    let owner_references = secret["metadata"]["ownerReferences"].as_array().unwrap();

    assert_eq!(owner_references.len(), 1);
    assert_eq!(
        owner_references[0]["name"].as_str().unwrap(),
        "idempotent-secrets"
    );
    assert_eq!(owner_references[0]["kind"].as_str().unwrap(), "ConfigMap");
}

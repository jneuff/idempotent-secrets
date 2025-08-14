mod helpers;
use std::process::{Command, Output};

use helpers::*;

struct IdempotentSecrets {
    secrets: Vec<String>,
    namespace: String,
}

impl IdempotentSecrets {
    fn in_namespace(namespace: &str) -> Self {
        Self {
            secrets: vec![],
            namespace: namespace.to_string(),
        }
    }

    fn with_secret(mut self, secret: impl Into<String>) -> Self {
        self.secrets.push(secret.into());
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
        .with_secret(r#"{"name":"secret-1", "type":"RandomString"}"#)
        .run()
        .unwrap();

    assert_no_errors(output);
    let secret_1 = kubectl_get_secret(namespace.name(), "secret-1").unwrap();

    let output = IdempotentSecrets::in_namespace(namespace.name())
        .with_secret(r#"{"name":"secret-1", "type":"RandomString"}"#)
        .run()
        .unwrap();

    assert_no_errors(output);
    let secret_2 = kubectl_get_secret(namespace.name(), "secret-1").unwrap();
    assert_eq!(secret_1, secret_2);
}

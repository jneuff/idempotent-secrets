use lazy_static::lazy_static;
use serde_json::Value;
use std::process::Command;
use std::sync::Arc;

pub struct Cluster {
    name: String,
}

impl Cluster {
    fn ensure() -> Self {
        let name = "idempotent-secrets-test".to_string();

        // Check if cluster already exists
        let output = Command::new("kind")
            .args(["get", "clusters"])
            .output()
            .expect("Failed to execute kind get clusters command");

        let clusters = String::from_utf8_lossy(&output.stdout);
        let cluster_exists = clusters.lines().any(|line| line.trim() == name);

        if !cluster_exists {
            // Create kind cluster
            let status = Command::new("kind")
                .args(["create", "cluster", "--name", &name])
                .status()
                .expect("Failed to execute kind command");

            assert!(status.success(), "Failed to create kind cluster");
        }

        Self { name }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn create_namespace(&self, name: &str) -> Result<(), kube::Error> {
        // Check if namespace exists and delete it if it does
        let output = Command::new("kubectl")
            .args(["get", "namespace", name])
            .output()
            .expect("Failed to execute kubectl get namespace command");

        if output.status.success() {
            let status = Command::new("kubectl")
                .args(["delete", "namespace", name, "--wait=true"])
                .status()
                .expect("Failed to execute kubectl delete namespace command");

            assert!(status.success(), "Failed to delete existing namespace");
        }

        // Create namespace
        let status = Command::new("kubectl")
            .args(["create", "namespace", name])
            .status()
            .expect("Failed to execute kubectl create namespace command");

        assert!(status.success(), "Failed to create namespace");

        Ok(())
    }
}

pub struct TestNamespace {
    name: String,
    _cluster: Arc<Cluster>,
}

impl TestNamespace {
    pub fn name(&self) -> &str {
        &self.name
    }
}

lazy_static! {
    pub static ref CLUSTER: Arc<Cluster> = Arc::new(Cluster::ensure());
}

pub fn namespace(name: &str) -> TestNamespace {
    let cluster = CLUSTER.clone();
    cluster
        .create_namespace(name)
        .expect("Failed to create namespace");

    TestNamespace {
        name: name.to_string(),
        _cluster: cluster,
    }
}

#[macro_export]
macro_rules! given_a_namespace {
    () => {{
        use k8s_test_utils::namespace;
        let test_name = stdext::function_name!()
            .rsplit("::")
            // async tests get wrapped in a closure
            .skip_while(|p| *p == "{{closure}}")
            .next()
            .unwrap()
            .replace("_", "-")
            .to_lowercase();
        namespace(&test_name)
    }};
}

pub fn kubectl_get_secret(namespace: &str, secret_name: &str) -> Result<Value, anyhow::Error> {
    let output = Command::new("kubectl")
        .args(["get", "secret", secret_name, "-n", namespace, "-ojson"])
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

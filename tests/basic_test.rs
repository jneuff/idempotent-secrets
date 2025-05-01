use lazy_static::lazy_static;
use std::process::Command;
use std::sync::Arc;

struct Cluster {
    name: String,
}

impl Cluster {
    fn new() -> Self {
        let name = "create-secret-test".to_string();

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

        if std::env::var("GITHUB_CI").is_err() {
            // Load docker image
            let status = Command::new("kind")
                .args([
                    "load",
                    "docker-image",
                    "create-secret:latest",
                    "--name",
                    &name,
                ])
                .status()
                .expect("Failed to execute kind load docker-image command");
            assert!(status.success(), "Failed to load docker image");
        }

        Self { name }
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

struct TestNamespace {
    name: String,
    cluster: Arc<Cluster>,
}

lazy_static! {
    static ref CLUSTER: Arc<Cluster> = Arc::new(Cluster::new());
}

fn namespace(name: &str) -> TestNamespace {
    let cluster = CLUSTER.clone();
    cluster
        .create_namespace(name)
        .expect("Failed to create namespace");

    TestNamespace {
        name: name.to_string(),
        cluster,
    }
}

macro_rules! given_a_namespace {
    () => {{
        let test_name = stdext::function_name!()
            .split("::")
            .skip(1)
            .next()
            .unwrap()
            .replace("_", "-")
            .to_lowercase();
        namespace(&test_name)
    }};
}

#[test]
fn test_namespace_creation() {
    let ns = given_a_namespace!();
    assert_eq!(ns.name, "test-namespace-creation");
    assert!(!ns.cluster.name.is_empty());
}

#[test]
fn test_helm_installation_and_secret_creation() {
    // let namespace = given_a_namespace!();
    let namespace = namespace("test-create-secret");

    let mut args = vec![
        "upgrade",
        "--install",
        "create-secret",
        "./helm/create-secret",
        "--namespace",
        &namespace.name,
        "--wait",
        "--wait-for-jobs",
        "--timeout",
        "1m",
    ];
    if std::env::var("GITHUB_CI").is_err() {
        args.extend(["--set", r#"image.registry="#]);
    }
    // Install Helm chart
    let status = Command::new("helm")
        .args(args)
        .status()
        .expect("Failed to execute helm upgrade command");

    assert!(status.success(), "Failed to install Helm chart");

    // Verify secret creation
    let output = Command::new("kubectl")
        .args(["get", "secret", "secret1", "-n", &namespace.name])
        .output()
        .expect("Failed to execute kubectl get secret command");

    assert!(output.status.success(), "Secret was not created");
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        output_str.contains("secret1"),
        "Secret 'secret1' not found in output"
    );
}

use lazy_static::lazy_static;
use std::process::Command;
use std::sync::Arc;

const HELM_COMMAND_TIMEOUT: &str = "60s";

fn image_tag_from_values() -> &'static str {
    let values = include_str!("../helm/create-secret/values.yaml");
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

fn set_image_tag() -> String {
    format!(r#"image.tag={}"#, image_tag())
}

struct Cluster {
    _name: String,
}

impl Cluster {
    fn ensure() -> Self {
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
                    &format!("create-secret:{}", image_tag()),
                    "--name",
                    &name,
                ])
                .status()
                .expect("Failed to execute kind load docker-image command");
            assert!(status.success(), "Failed to load docker image");
        }

        Self { _name: name }
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
    _cluster: Arc<Cluster>,
}

lazy_static! {
    static ref CLUSTER: Arc<Cluster> = Arc::new(Cluster::ensure());
}

fn namespace(name: &str) -> TestNamespace {
    let cluster = CLUSTER.clone();
    cluster
        .create_namespace(name)
        .expect("Failed to create namespace");

    TestNamespace {
        name: name.to_string(),
        _cluster: cluster,
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
fn test_helm_installation_and_secret_creation() {
    let namespace = given_a_namespace!();
    let secret_name = "rsa-key";
    let set_image_tag = set_image_tag();

    let mut args = vec![
        "upgrade",
        "--install",
        "create-secret",
        "./helm/create-secret",
        "--namespace",
        &namespace.name,
        "--set",
        &set_image_tag,
        "--set",
        r#"secret.name="rsa-key""#,
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
    let output = Command::new("kubectl")
        .args(["get", "secret", secret_name, "-n", &namespace.name])
        .output()
        .expect("Failed to execute kubectl get secret command");

    assert!(output.status.success(), "Secret was not created");
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        output_str.contains(secret_name),
        "Secret '{secret_name}' not found in output"
    );
}

#[test]
fn should_adhere_to_pod_security_standards() {
    let namespace = given_a_namespace!();
    let set_image_tag = set_image_tag();
    // label namespace with pod-security.kubernetes.io/enforce: privileged
    let status = Command::new("kubectl")
        .args([
            "label",
            "namespace",
            &namespace.name,
            "pod-security.kubernetes.io/enforce=restricted",
        ])
        .status()
        .expect("Failed to execute kubectl label namespace command");

    assert!(status.success(), "Failed to label namespace");

    let mut args = vec![
        "install",
        "create-secret",
        "./helm/create-secret",
        "--namespace",
        &namespace.name,
        "--set",
        &set_image_tag,
        "--set",
        r#"secret.name="rsa-key""#,
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
}

#[test]
fn should_install_two_releases_with_different_names() {
    let namespace = given_a_namespace!();
    let set_image_tag = set_image_tag();

    let mut args = vec![
        "install",
        "create-secret",
        "./helm/create-secret",
        "--namespace",
        &namespace.name,
        "--set",
        &set_image_tag,
        "--set",
        r#"secret.name="rsa-key""#,
        "--wait",
        "--wait-for-jobs",
        "--timeout",
        "30s",
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

    let mut args = vec![
        "install",
        "create-secret-2",
        "./helm/create-secret",
        "--namespace",
        &namespace.name,
        "--set",
        &set_image_tag,
        "--set",
        r#"secret.name="rsa-key-2""#,
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

    assert!(
        status.success(),
        "Failed to install Helm chart a second time"
    );
}

#[test]
fn should_allow_fullname_override() {
    let namespace = given_a_namespace!();
    let set_image_tag = set_image_tag();

    let mut args = vec![
        "install",
        "create-secret",
        "./helm/create-secret",
        "--namespace",
        &namespace.name,
        "--set",
        &set_image_tag,
        "--set",
        r#"secret.name="rsa-key""#,
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
        .args(["get", "pod", "-n", &namespace.name])
        .output()
        .expect("Failed to execute kubectl get pod command");

    assert!(output.status.success(), "Failed to get pod");
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        output_str.contains("custom-name"),
        "Pod name does not contain custom-name"
    );

    let output = Command::new("kubectl")
        .args(["get", "serviceaccount", "-n", &namespace.name])
        .output()
        .expect("Failed to execute kubectl get serviceaccount command");

    assert!(output.status.success(), "Failed to get serviceaccount");
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        output_str.contains("custom-name"),
        "Serviceaccount name does not contain custom-name"
    );

    let output = Command::new("kubectl")
        .args(["get", "role", "-n", &namespace.name])
        .output()
        .expect("Failed to execute kubectl get role command");

    assert!(output.status.success(), "Failed to get role");
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        output_str.contains("custom-name"),
        "Role name does not contain custom-name"
    );

    let output = Command::new("kubectl")
        .args(["get", "rolebinding", "-n", &namespace.name])
        .output()
        .expect("Failed to execute kubectl get rolebinding command");

    assert!(output.status.success(), "Failed to get role");
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        output_str.contains("custom-name"),
        "Role name does not contain custom-name"
    );
}

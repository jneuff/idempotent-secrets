# Idempotent Secrets

Idempotently create secrets in a Kubernetes namespace. We currently support the following secret
types:

- `RsaKeypair` a RSA keypair with a fixed key length of 4096.
- `RandomString` a random Base64 encoded string of a fixed length of 128 bytes (prior to encoding).

## Usage Example

```sh
# Create namespace
$ kubectl create namespace demo
namespace/demo created

# Define secrets
$ cat <<EOF > values.yaml
secrets:
  - name: secret-1
    type: RsaKeypair
EOF

# Install Helm chart
$ helm install idempotent-secrets oci://ghcr.io/jneuff/idempotent-secrets/helm/idempotent-secrets --namespace demo --values ./values.yaml
Pulled: ghcr.io/jneuff/idempotent-secrets/helm/idempotent-secrets:0.1.1
Digest: sha256:b24600379b65502ee9b4d784734a8aaa9b02dff2a5297a4a111ade0c7167fc0f
NAME: idempotent-secrets
LAST DEPLOYED: Wed Jul 23 13:46:48 2025
NAMESPACE: demo
STATUS: deployed
REVISION: 1
TEST SUITE: None

# Inspect the secret
kubectl get secrets --namespace demo secret-1 -oyaml
apiVersion: v1
data:
  private_key: LS0..S0K
  public_key: LS0..g==
kind: Secret
metadata:
  creationTimestamp: "2025-07-23T12:01:47Z"
  name: secret-1
  namespace: demo
  resourceVersion: "124697"
  uid: 22e66133-6d4a-4d91-b5e0-9ef5460f6a00
type: Opaque
```

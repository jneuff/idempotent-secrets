# Create a kwok cluster for testing
create-cluster:
    kwokctl create cluster --name test-cluster --wait 60s
    kubectl get nodes

# Delete the kwok cluster
delete-cluster:
    kwokctl delete cluster --name test-cluster

# Run the tests (depends on create-cluster)
test: delete-cluster create-cluster
    cargo test

build-container:
    docker build . -f ./Containerfile -t create-secret:latest

delete-namespace:
    kubectl delete namespace test-create-secret || true

create-namespace:
    kubectl create namespace test-create-secret

# Install Helm chart in the kind cluster
install-helm-chart:
    helm upgrade --install create-secret ./helm/create-secret \
    --namespace test-create-secret \
    --create-namespace \
    --set image.registry="" \
    --wait \
    --wait-for-jobs \
    --timeout 1m

# Verify secret creation
verify-secret:
    kubectl get secret -n test-create-secret secret1

# Run full integration test suite
integration-test: build-container delete-namespace create-namespace install-helm-chart verify-secret
    @echo "Integration test completed"

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
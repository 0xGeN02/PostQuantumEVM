# PostQuantumEVM — Kubernetes Deployment
#
# Deploys a 3-validator PoA network on Kubernetes.
#
# Architecture:
#   - pq-validator-{1,2,3}: PoA validators (round-robin ML-DSA-65 sealed blocks)
#   - pq-rpc: LoadBalancer/NodePort service exposing JSON-RPC
#
# Prerequisites:
#   1. Build and push Docker image: docker build -t <registry>/pqevm-node:latest -f Dockerfile.pq-reth .
#   2. Generate validator keys: ./scripts/generate-validator-keys.sh
#   3. Create secrets from generated keys (see below)
#
# Usage:
#   kubectl apply -f e2e/k8s/
#   kubectl get pods -n pqevm
#   kubectl logs -f pq-validator-1-0 -n pqevm
#
# To run E2E validation:
#   RPC=$(kubectl get svc pq-rpc -n pqevm -o jsonpath='{.status.loadBalancer.ingress[0].ip}')
#   cargo run --bin pq-e2e -- --rpc http://$RPC:8545
#
# Cleanup:
#   kubectl delete namespace pqevm

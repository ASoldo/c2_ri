# ArgoCD Setup

1. Install ArgoCD in your cluster:

```sh
kubectl create namespace argocd
kubectl apply -n argocd -f https://raw.githubusercontent.com/argoproj/argo-cd/stable/manifests/install.yaml
```

2. Apply the ArgoCD project + application:

```sh
kubectl apply -f k8s/argocd/project.yaml
kubectl apply -f k8s/argocd/c2-dev.yaml
```

Switch to staging/prod by applying the appropriate application manifest.

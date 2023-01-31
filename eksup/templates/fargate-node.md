### Fargate Node

ℹ️ [Fargate pod patching](https://docs.aws.amazon.com/eks/latest/userguide/fargate-pod-patching.html)

#### Upgrade

To update a Fargate node, you simply need to remove the existing node(s) and EKS will schedule new nodes using the appropriate Kubernetes version.
The Kubernetes version used by Fargate nodes is referenced from the control plane version at the time the node is created. Once the control plane has been updated, any new Fargate nodes created will use the latest patch version for the associated control plane version.

1. To update the Fargate node(s) used, use the Kubernetes [eviction API](https://kubernetes.io/docs/concepts/scheduling-eviction/api-eviction/) to evict the node while respecting `PodDisruptionBudgets` and `terminationGracePeriodSeconds`.

    Ensure you have updated your `kubeconfig` locally before executing the following commands:

    ```sh
    aws eks update-kubeconfig --region {{ region }}  --name {{ cluster_name }}
    ```

    Fargate nodes are identified by their `fargate-*` name prefix.

    ```sh
    kubectl get nodes | grep '\bfargate-'
    ```

    Drain the node to ensure the `PodDisruptionBudgets` and `terminationGracePeriodSeconds`

    ```sh
    kubectl drain <FARGATE-NODE> --delete-emptydir-data
    ```

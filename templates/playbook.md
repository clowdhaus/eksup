# EKS Cluster Upgrade: {{ current_version }} -> {{ target_version }}

### Table of Contents

- [Caveats](#caveats)
- [References](#references)
- [Pre-Upgrade](#pre-upgrade)
- [Upgrade](#upgrade)
  - [Upgrade the Control Plane](#upgrade-the-control-plane)
  - [Upgrade the Data Plane](#upgrade-the-data-plane)
  - [Upgrade Addons](#upgrade-addons)
- [Post-Upgrade](#post-upgrade)

## Caveats

- Unless specifically called out, the phrase `Amazon EKS cluster` or just `cluster` throughout this document refers to the control plane.
- If the Amazon EKS cluster primary security group has been deleted, the only course of action to upgrade is to create a new cluster and migrate your workloads.

    ```sh
    <aws eks describe-cluster - get primary security group then do a describe on it>
    ```

- In-place cluster upgrades can only be upgraded to the next incremental minor version. For example, you can upgrade from Kubernetes version 1.20 to 1.21, but not from 1.20 to 1.22.
- Reverting an upgrade, or downgrading the Kubernetes version, is not supported. If you upgrade your cluster to a new Kubernetes version and then want to revert to the previous version, you must create a new cluster and migrate your workloads.
- When updgrading the control plane, Amazon EKS performs standard infrastructure and readiness health checks for network traffic on the new control plane nodes to verify that they're working as expected. If any of these checks fail, Amazon EKS reverts the infrastructure deployment, and your cluster control plane remains on the prior Kubernetes version. Running applications aren't affected, and your cluster is never left in a non-deterministic or unrecoverable state. Amazon EKS regularly backs up all managed clusters, and mechanisms exist to recover clusters if necessary.

## References

Before upgrading, you should review the following resources:

- [Updating an Amazon EKS cluster Kubernetes version](https://docs.aws.amazon.com/eks/latest/userguide/update-cluster.html)
{{#if k8s_deprecation_url }}
- [Kubernetes `{{ target_version }}` API deprecations]({{ k8s_deprecation_url }})
{{/if}}
- [Kubernetes `{{ target_version }}` release announcement]({{ k8s_release_url }})
- [EKS `{{ target_version }}` release notes](https://docs.aws.amazon.com/eks/latest/userguide/kubernetes-versions.html#kubernetes-{{ target_version }})

## Pre-Upgrade

1. Compare the Kubernetes version of your cluster control plane to the Kubernetes version of your nodes. Before updating your control plane to a new Kubernetes version, make sure that the Kubernetes minor version of both the managed nodes and Fargate nodes in your cluster are the same as your control plane's version.

    Control plane Kubernetes version:
    ```sh
    kubectl version --short
    ```

    Nodes Kubernetes version:
    ```sh
    kubectl get nodes
    ```

2. Verify that there are at least 5 free IPs in the VPC subnets used by the control plane. Amazon EKS creates new cluster elastic network interfaces (network interfaces) in any of the subnets specified for the control plane.

    ```sh
    aws ec2 describe-subnets --subnet-ids $(aws eks describe-cluster --name <CLUSTER_NAME> \
      --query 'cluster.resourcesVpcConfig.subnetIds' --output text) --query 'Subnets[*].AvailableIpAddressCount'
    ```

3. Check that the security groups allow the necessry cluster communication

    - Cluster primary security group should still be present:

        ```sh
        aws ec2 describe-security-groups --group-ids $(aws eks describe-cluster --name <CLUSTER_NAME> \
          --query 'cluster.resourcesVpcConfig.clusterSecurityGroupId' --output text)
        ```

    - The new control plane network interfaces may be created in different subnets than what your existing control plane network interfaces are in, so make sure that your security group rules allow the [required cluster communication](https://docs.aws.amazon.com/eks/latest/userguide/sec-group-reqs.html) for any of the subnets that you specified when you created your cluster.

4. Check Kubernetes version prerequisites

    - ⚠️ You can utilize https://github.com/FairwindsOps/pluto to scan your Kubernetes manifests for deprecated/removed API versions

## Upgrade

The steps to upgrade an Amazon EKS cluster can be summarized as:

1. Upgrade the control plane
2. Upgrade the data plane
3. Upgrade addons (`kube-proxy`, `coredns`, `vpc-cni`, `cluster-autoscaler`, etc.)

### Upgrade the Control Plane

1. Upgrade the control plane to the next Kubernetes minor version:

    ```sh
    aws eks update-cluster-version --name <CLUSTER_NAME> --kubernetes-version {{ target_version }}
    ```

2. Wait for the control plane to finish upgrading before proceeding with any further modifications

### Upgrade the Data Plane

{{ self_managed_node_group }}

{{ eks_managed_node_group }}

{{ fargate_profile }}

### Upgrade Addons

## Post Upgrade

- ⚠️ Update applications running on the cluster
- ⚠️ Update tools that interact with the cluster (kubectl, awscli, etc.)
- ⚠️ TODO

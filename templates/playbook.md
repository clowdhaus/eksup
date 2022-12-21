# EKS Cluster Upgrade: {{ current_version }} -> {{ target_version }}

### Table of Contents

- [Pre-Upgrade](#pre-upgrade)
- [Upgrade](#upgrade)
  - [Upgrade the Control Plane](#upgrade-the-control-plane)
  - [Upgrade the Data Plane](#upgrade-the-data-plane)
  - [Upgrade Addons](#upgrade-addons)
- [Post-Upgrade](#post-upgrade)
- [References](#references)

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
    aws ec2 describe-subnets --subnet-ids $(aws eks describe-cluster --name {{ cluster_name }} --region <REGION> \
      --query 'cluster.resourcesVpcConfig.subnetIds' --output text) --region <REGION> --query 'Subnets[*].AvailableIpAddressCount'
    ```

3. Check that the security groups allow the necessry cluster communication

    - ⚠️ If the current cluster primary security group was deleted, then only route is blue/green upgrade
    - ⚠️ What steps/actions do we provide here?

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
    aws eks update-cluster-version --region <REGION> --name <CLUSTER_NAME> --kubernetes-version {{ target_version }}
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

## References

{{#if k8s_deprecation_url }}
- [Kubernetes `{{ target_version }}` API deprecations]({{ k8s_deprecation_url }})
{{/if}}
- [Kubernetes `{{ target_version }}` release announcement]({{ k8s_release_url }})
- [EKS `{{ target_version }}` release notes](https://docs.aws.amazon.com/eks/latest/userguide/kubernetes-versions.html#kubernetes-{{ target_version }})

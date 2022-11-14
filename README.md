# eksup

## Why is this needed

Kubernetes releases a new version [approximately every 4 months](https://kubernetes.io/releases/release/). Each minor version is supported for 12 months after it's first released by the Kubernetes community, and Amazon EKS supports a Kubernetes version for 14 months once made available on Amazon EKS. In line with the Kubernetes community support for Kubernetes versions, Amazon EKS is committed to supporting at least four versions of Kubernetes at any given time.This means that Amazon EKS users need to be prepared to upgrade their cluster version(s) at least once a year.

### What is it

This CLI produces a cluster upgrade playbook that attempts to:

- Educate users on the overall process of upgrading an Amazon EKS cluster (order of operations, which parts AWS manages and which parts are the user's respopnsibility, etc.)
- Provide one approach as the basis for upgrading a cluster that users can modify/customize to suit their cluster configuration/architecture and business requirements
- Provide recommendations on what to check for and precautions to consider before upgrading, how to perform the cluster upgrade, and considerations for configuring your cluster and/or applications to minimize risk and disruption during the upgrade process

The end goal of this tool is a playbook that you and your team feel confident in executing repeatedly each upgrade cycle. After each upgrade, reflect on the process with your team and capture any learnings so that you can continuously improve and build confidence in the upgrade process.

### What it is NOT

- This CLI does not access your cluster(s) or perform any actions on your behalf
- The guidance and recommendations are not exhaustive. The information provided here is intended to be broadly applicable to the majority of Amazon EKS users. However, there are many factors that can affect your cluster upgrade process, specifically with regards to the applications running on the cluster and their configurations, and users will need to consider these factors when planning their upgrade process. This is why the output from this tool is a playbook that is intended to be modified and tailored to your cluster's configurations, applications, workloads, business requirements, processes, etc. As always, it is strongly recommended to practice your upgrade process in a non-production environment before attempting to upgrade your production cluster(s).

## Pre

### Info

- [Amazon EKS version](https://docs.aws.amazon.com/eks/latest/userguide/kubernetes-versions.html)
- [Kubernetes deprecation guide](https://kubernetes.io/docs/reference/using-api/deprecation-guide)
- [Kubernetes changelog](https://github.com/kubernetes/kubernetes/blob/master/CHANGELOG/CHANGELOG-1.24.md)

### Actions

1. Check version skew between data plane and control plane

  - Can provide an awscli command for users to use for EKS optimized AMI
  - Can also provide kubectl commands to check on cluster

2. Check that there are at least 5 free IPs in the VPC subnets
3. Check that the security groups allow the necessary cluster communication

  - If the current cluster primary security group was deleted, then only route is blue/green upgrade

4. Check Kubernetes version prerequisites

  - v1.22 -> https://docs.aws.amazon.com/eks/latest/userguide/update-cluster.html#update-1.22
  - https://kubernetes.io/docs/reference/using-api/deprecation-guide

## Upgrade

1. Here is the high level steps to upgrade EKS cluster
   a. Upgrade the control plane
   b. Upgrade the data plane
    - [EKS Managed node groups](https://docs.aws.amazon.com/eks/latest/userguide/update-managed-node-group.html)
    - [Self-managed node groups](https://docs.aws.amazon.com/eks/latest/userguide/update-workers.html)
    - Fargate profiles: Any new pods that are launched on Fargate have a kubelet version that matches your cluster version. Existing Fargate pods aren't changed.
      c. Upgrade addons (kube-proxy, coredns, vpc-cni, cluster-autoscaler, etc.)
    - List those that have callouts tied to versions. See EKS docs
      d. [Optional] Update applications running on the cluster as needed
      e. [Optional] Re-run `popeye` to check for any new deprecations and remediate
      f. [Optional] Update CLI versions
    - kubectl
    - awscli (v1alpha1 -> v1beta1 for `aws eks update-kubeconfig`)

## Post

## Future

- Have users point CLI at the cluster and just scan for information to reduce amount of input
  - As part of playbook, we can use things like `popeye` to generate action items for pre-upgrade
  - Maybe it cannot generate the playbook, but it could pre-populate a lot of the information for users who can do a final review on (generate a config)
- Add snippets for commonly used provisioning tools to explain how those fit into the guidance
  - <Select> Framework used to managed EKS cluster [`terraform-aws-eks`, `eksctl`]
  - <Select> Version of framework used [`v18.x`, `v19.x`]

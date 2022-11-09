# eksup

- For each "step" or bit of information, we'll need evaluate if
  - This step is generic across all versions of Kubernetes
  - This step is specific to a version of Kubernetes. For this, we'll need an enum like structure to populate the information correctly
- In the generated playbook, a brief synposis or overview should be provided at the top to give a high level overview of what the playbook is doing

## Why is this needed?

Kubernetes releases a new version [approximately every 4 months](https://kubernetes.io/releases/release/), and Amazon EKS provides support for a rolling window of 4 versions (typically providing support within 6 months of upstream release). Users have stated that they simply aren't prepared to handle the constant wave of Kubernetes releases nor are they full versed on what steps to take, what they need to watch out for, what changes are required to upgrade, etc., while ensuring that their services running on the cluster(s) avoid

## Inputs

- Cluster current version
- Upgrade strategy:
  - `in-place` (default)
  - `blue/green`]
  - To start, this is not an option and only `in-place` is supported. A future version will support `blue/green` upgrades
- Workload classifications
  - `stateful`
  - `multi-tenant`

### Prompts

- Only valid if the upgrade strategy is `in-place`
  - Data plane: EKS MNG, Self-MNG, Fargate profile (mix?)
  - Autoscaling: Cluster autoscaler, Karpenter, none, other
- Only vaid if the upgrade strategy is `blue/green`
  - How is DNS currently managed (i.e. - `external-dns`)
  - Current ingress setup - AWS LBC, Nginx ingress, Istio gateway, ALB vs NLB, API Gateway, etc.
    - There is a relationship between the ingress setup and the DNS setup as well as various methods to manage the ingress AWS resources (cluster, external)

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
3. Check that the security groups allow the necessry cluster communication
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
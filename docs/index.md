# EKS Cluster Upgrade Guidance

## Why Is This Needed

Kubernetes releases a new version [approximately every 4 months](https://kubernetes.io/releases/release/). Each minor version is supported for 12 months after it's first released by the Kubernetes community, and Amazon EKS supports a Kubernetes version for 14 months once made available. In line with the Kubernetes community support for versions, Amazon EKS is committed to supporting at least four versions of Kubernetes at any given time. This means that Amazon EKS users need to be prepared to upgrade their cluster version(s) at least once a year. However, there are a number of factors that can make each upgrade different and unique that users will need to evaluate prior to each upgrade. Factors that can change between each upgrade cycle include:

- New team members who are inexperienced with the upgrade process, and/or prior team members who have experience in cluster upgrades are no longer on the team
- Different Kubernetes APIs are marked as deprecated or removed from the next release
- Kubernetes resources that were previously provided by Kubernetes "in-tree" are now provided as external resources (i.e - moving Kubernetes in-tree cloud provider code out to their respective standalone projects such as ALB ingress controller "in-tree" to the external ALB load balancer controller)
- Various changes and deprecations in the components used by Kubernetes (i.e - moving from `kube-dns` to `CoreDNS`, moving from Docker engine to `containerd` for container runtime, dropping support for `dockershim`, etc.)
- Changes in your applications, your architecture, or the amount of traffic your clusters are handling. Over time, the number of available IPs for the cluster resources may shrink, stateful workloads may have been added to the cluster, etc., and these factors can influence the upgrade process.

### What Is `eksup`

`eksup` is a CLI that helps users prepare for a cluster upgrade - providing users as much relevant information as possible for their upgrade.

`eksup` gives users the ability to analyze their cluster(s) against the next version of Kubernetes, highlighting any findings that may affect the upgrade process. In addition, `eksup` has the ability to generate a playbook tailored to the cluster analyzed that provides the process for upgrading the cluster including the findings that require remediation. The playbook output allows users to edit the upgrade steps to suit their cluster configuration and business requirements plus capture any specific learnings during the upgrade process. Since most users typically perform upgrades on nonproduction clusters first, any additional steps or call-outs that are discovered during the upgrade process can be captured and used to improve the upgrade process for their production clusters. Users are encouraged to save their playbooks as historical artifacts for future reference to ensure that with each cycle, the team has a better understanding of the upgrade process and more confidence in swiftly working through cluster upgrades before their Kubernetes version support expires.

### What It Is NOT

- `eksup` is not a tool that will perform the cluster upgrade. It is assumed that clusters are generally created using an infrastructure as code approach through tools such as Terraform, `eksctl`, or CloudFormation. Therefore, users are encouraged to use those tools to perform the upgrade to avoid any resource definition conflicts.
- It does not perform any modifications on the resources it identifies as needing, or recommending, changes. Again, following the approach of infrastructure as code, users are encouraged to make these changes through their normal change control process at the appropriate time in the upgrade process.
  - In the future, `eksup` may provide functionality to help in converting a Kubernetes manifest definition from one API version to the next. However, this will occur on the users local filesystem and not against a live cluster. `eksup` will always operate from the perspective of infrastructure as code; any feature requests that support this tenant are encouraged.

### Symbol Table

| Symbol | Description |
| :----: | :---------- |
| ℹ️     | Informational - users are encouraged to familiarize themselves with the information but no action is required to upgrade  |
| ⚠️     | Recommended - users are encouraged to evaluate the recommendation and determine if it is applicable and whether or not to act upon that recommendation. Not remediating the finding does not prevent the upgrade from occurring. |
| ❌     | Required - users must remediate the finding prior to upgrading to be able to perform the upgrade and avoid downtime or disruption |

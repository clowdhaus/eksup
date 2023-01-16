# eksup

## Why Is This Needed

Kubernetes releases a new version [approximately every 4 months](https://kubernetes.io/releases/release/). Each minor version is supported for 12 months after it's first released by the Kubernetes community, and Amazon EKS supports a Kubernetes version for 14 months once made available on Amazon EKS. In line with the Kubernetes community support for Kubernetes versions, Amazon EKS is committed to supporting at least four versions of Kubernetes at any given time. This means that Amazon EKS users need to be prepared to upgrade their cluster version(s) at least once a year. However, there are a number of factors that can make each upgrade different and unique. Factors that can change between each upgrade cycle include:

- New team members who are inexperienced with the upgrade process, and/or prior team members who have experience in cluster upgrades are no longer on the team
- Different Kubernetes APIs are marked as deprecated or removed from the next release
- Kubernetes resources that were previously provided by Kubernetes "in-tree" are now provided as external resources (i.e - moving Kubernetes in-tree cloud provider code out to their respective standalone projects such as ALB ingress controller "in-tree" to the external ALB load balancer controller)
- Various changes and deprecations in the components used by Kubernetes (i.e - moving from `kube-dns` to `CoreDNS`, moving from Docker engine to `containerd` for container runtime, dropping support for `dockershim`, etc.)
- Changes in your applications, your architecture, or the amount of traffic your clusters are handling. As teams and organizations grow, clusters will grow and change as well, consuming more resources and potentially pushing some of the limits of what your previous architecture was designed to support (i.e. - number of available IPs might become constrained, stateful workloads may have been added to the cluster, etc.)

### What It Is

`eksup` is a CLI that helps users prepare for a cluster upgrade - providing users as much relevant information as possible for their upgrade.

`eksup` gives users the ability to analyze their cluster(s) against the next version of Kubernetes and generate a playbook that provides both the steps to upgrade the cluster as well as the relevant information users should be aware of. This gives teams the ability to prepare their clusters for the upgrade based on the information provided in the analysis, as well as a means to practice and rehearse their upgrade process starting with a development or sandbox environment cluster, working their way up through their environments towards production. Any learnings discovered during this rehearsal and rollout process can be captured and used to improve the upgrade process for both the current cycle as well as the next. Users are encouraged to save their playbooks as historical artifacts for future reference to ensure that with each cycle, the team has a better understanding of the upgrade process and more confidence in swiftly working through cluster upgrades before their Kubernetes version support expires.

This CLI produces a cluster upgrade playbook that attempts to:

- Educate users on the overall process of upgrading an Amazon EKS cluster (order of operations, which parts AWS manages and which parts are the user's responsibility, etc.)
- Provide one approach as the basis for upgrading a cluster that users can modify/customize to suit their cluster configuration/architecture and business requirements
- Provide recommendations on what to check for and precautions to consider before upgrading, how to perform the cluster upgrade, and considerations for configuring your cluster and/or applications to minimize risk and disruption during the upgrade process

The end goal of this tool is a playbook that you and your team feel confident in executing repeatedly each upgrade cycle. After each upgrade, reflect on the process with your team and capture any learnings so that you can continuously improve and build confidence in the upgrade process.

### What It Is NOT

- `eksup` is not a tool that will perform the cluster upgrade. It is assumed that clusters are generally created using an infrastructure as code approach through tools such as Terraform, `eksctl`, CloudFormation, etc., and therefore users are encouraged to use those tools to perform the upgrade to avoid any resource definition conflicts.
- It does not perform any modifications on the live resources it identifies as needing, or recommending, changes. Again, following the approach of infrastructure as code, users are encouraged to make these changes through their normal change control process at the appropriate time (either before or after upgrading the cluster).
  - In the future, `eksup` may provide functionality to help in converting a Kubernetes resource from one API version to the next. However, this will occur on the users local filesystem and not against a live cluster. `eksup` will always operate from the perspective of infrastructure as code; any feature requests that support this tenant are encouraged.

## Commands

- `analyze`
  - text stdout for quick analysis from CLI
  - `--output-format json --output-type file`: JSON stdout for data collection and reporting from a central location (CronJob), text stdout for quick analysis from CLI
- `create`
  - `playbook`
    - a playbook with generic upgrade steps; informative process on cluster upgrade without need to have a cluster or access a cluster
    - `--with-analysis`: playbook with the analysis results; most concrete set of information on the current cluster state with guidance
- `migrate` or `transform`
  - [Future - TBD] Given a manifest, convert the manifest to the next, stable API version. Some resources only need the API version changed, others will require the schema to be modified to match the new API version

### Analyze Checks

#### Key

- ℹ️  Informational: Users should be aware, but it is not a hard requirement for upgrading
- ❌  Required: Users are strongly encouraged to address prior to upgrade to avoid any potential issues

| Type | Description
| :--: | :-------------------------------------------------------------------------------------- |
| ❌ | The control plane version matches the version used by the data plane |
| ❌ | At least 5 available IPs for the control plane to upgrade; required for cross account ENI creation |
| ℹ️ | Sufficient available IPs for the nodes to support the surge, in-place rolling upgrade. Irrespective of Kubernetes, each EC2 instance |
| ℹ️ | Sufficient available IPs for the pods to support the surge, in-place rolling upgrade. This check is used when custom networking is enabled since the IPs used by pods are coming from subnets different from those used by the EC2 instances themselves |
| ❌ | EKS addon(s) are compatible with the next Kubernetes version |
| ❌ | No health issues reported for the EKS cluster (control plane) |
| ❌ | No health issues reported for the EKS managed node groups. There aren't any available health statuses available from the AWS API for self-managed node groups or Fargate profiles at this time |
| ❌ | No health issues reported for the EKS addons |
| ℹ️ | EKS managed node group(s) are using latest launch template version; no pending updates |
| ℹ️ | Self-managed node group(s) are using latest launch template version; no pending updates |

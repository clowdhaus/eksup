# eksup

## Why is this needed

Kubernetes releases a new version [approximately every 4 months](https://kubernetes.io/releases/release/). Each minor version is supported for 12 months after it's first released by the Kubernetes community, and Amazon EKS supports a Kubernetes version for 14 months once made available on Amazon EKS. In line with the Kubernetes community support for Kubernetes versions, Amazon EKS is committed to supporting at least four versions of Kubernetes at any given time.This means that Amazon EKS users need to be prepared to upgrade their cluster version(s) at least once a year.

### What is it

This CLI produces a cluster upgrade playbook that attempts to:

- Educate users on the overall process of upgrading an Amazon EKS cluster (order of operations, which parts AWS manages and which parts are the user's responsibility, etc.)
- Provide one approach as the basis for upgrading a cluster that users can modify/customize to suit their cluster configuration/architecture and business requirements
- Provide recommendations on what to check for and precautions to consider before upgrading, how to perform the cluster upgrade, and considerations for configuring your cluster and/or applications to minimize risk and disruption during the upgrade process

The end goal of this tool is a playbook that you and your team feel confident in executing repeatedly each upgrade cycle. After each upgrade, reflect on the process with your team and capture any learnings so that you can continuously improve and build confidence in the upgrade process.

### What it is NOT

- This CLI does not access your cluster(s) or perform any actions on your behalf
  - ⚠️ Need to revisit this - we should do more than just generate a Markdown file and tell users to use other OSS tools and piece together the results. This CLI should be all encompassing for EKS upgrades to give users as much details information as possible on their cluster to give the best upgrade experience and the most confidence in the upgrade process.
- The guidance and recommendations are not exhaustive. The information provided here is intended to be broadly applicable to the majority of Amazon EKS users. However, there are many factors that can affect your cluster upgrade process, specifically with regards to the applications running on the cluster and their configurations, and users will need to consider these factors when planning their upgrade process. This is why the output from this tool is a playbook that is intended to be modified and tailored to your cluster's configurations, applications, workloads, business requirements, processes, etc. As always, it is strongly recommended to practice your upgrade process in a non-production environment before attempting to upgrade your production cluster(s).
  - ⚠️ Need to revisit this - we should strive to remove as much ambiguity as possible

## Notes

<These will be removed eventually>

Choices:
- CLI / Terraform
- [EKS MNG] default AMI vs custom AMI (???)

- Prefer topology hints over affinity for larger clusters
  - [Inter-pod affinity and anti-affinity](https://kubernetes.io/docs/concepts/scheduling-eviction/assign-pod-node/#inter-pod-affinity-and-anti-affinity)
> Note: Inter-pod affinity and anti-affinity require substantial amount of processing which can slow down scheduling in large clusters significantly. We do not recommend using them in clusters larger than several hundred nodes.

Helpful commands:

- `kubectl api-groups`
- `kubectl api-resources -o wide`

## Questions

- What is the guidance for batch workloads?
- What is the recommended way to manage the lifecycle of Fargate profiles?
  - Best way to "roll" profiles after the control plane Kubernetes version has been upgraded
- What is the churn calculation for updating node groups?
  - When updating a self-managed node group, how many instances are spun up before instances are terminated, whats the lifecycle, etc.
  - Same for EKS managed node group - how much do we surge to (max), etc.
  - This is important for:
    - Do users have enough resources at their disposal before the start their upgrade or do they need to request resource limit increases (EC2s)?
    - We state that the control plane needs at least 5 free IPs before it can be upgraded, but this also will affect the data plane upgrade and churn
    - How long will the upgrade take users?
    - How can users influence the amount of churn - why should they, what recommendations or guidance do we have?
- Do we have different guidance for large clusters?
  - See note on [Inter-pod affinity and anti-affinity](https://kubernetes.io/docs/concepts/scheduling-eviction/assign-pod-node/#inter-pod-affinity-and-anti-affinity)

## Commands

- `eksup create-playbook` - Creates a cluster upgrade playbook
- `eksup analyze` - Analyzes a cluster and provides feedback based on pre-upgrade checks/considerations

### Checks

- The control plane version matches the version used by the data plane.
  - Hard requirement
- There are at least 5 avaialable IPs for the control plane to upgrade; requried for cross account ENI creation
  - Hard requirement
- There are sufficient available IPs for the nodes to support the surge, in-place rolling upgrade. Irrespective of Kubernetes, each EC2 instance requires at least 1 IP to launch.
  - Informational notice
- There are sufficient available IPs for the pods to support the surge, in-place rolling upgrade. This check is used when custom networking is enabled since the IPs used by pods are coming from subnets different from those used by the EC2 instances themselves.
  - Informational notice
- The current EKS addon(s) are compatible with the next Kubernetes version
  - Hard requirement
  - This will also show where the addon version stands relative to the default and latest versions for the current Kubernetes version as well as the Kubernetes version the upgrade is targetting
- There are no health issues reported for the EKS cluster (control plane)
  - Hard requirement
- There are no health issues reported for the EKS managed node groups. There aren't any available health statuses available from the AWS API for self-managed node groups or Fargate profiles at this time
  - Hard requirement
- There are no health issues reported for the EKS addons
  - Hard requirement

#### TBD Checks

- APIs deprecated and/or removed in the next Kubernetes version
  - For now, recommend `pluto` or `kubent`
  - Add section on how those tools work, what to watch out for (asking the API Server is not trustworthy, scanning manifests directly is the most accurate)

### Output Levels

1. `--quiet` - suppress all output
2. (default, no flags) - show failed checks on hard requirements
3. `--warn` - in addition to failed, show warnings (low number of IPs available for nodes/pods, addon version older than current default, etc.)
4. `--info` - in addition to failed and warnings, show informational notices (number of IPs available for nodes/pods, addon version relative to current default and latest, etc.)

### ToDo

- [ ] PDBs set on deployments/replicasets and statefulsets
- [ ] Multiple replicas specified on deployments/replicasets and statefulsets
- [ ] Detect docker socket use (1.24+ affected) https://github.com/aws-containers/kubectl-detector-for-docker-socket
- [ ] Warn on pod security policy use (deprecated 1.21, removed 1.25) https://kubernetes.io/docs/concepts/security/pod-security-policy/
  - [ ] Advise to switch to pod security admission https://kubernetes.io/docs/concepts/security/pod-security-admission/
- [ ] Something for https://kubernetes.io/blog/2021/12/10/storage-in-tree-to-csi-migration-status-update/ ?
- [ ] The [in-tree Amazon EBS storage provisioner](https://kubernetes.io/docs/concepts/storage/volumes/#awselasticblockstore) is deprecated. If you are upgrading your cluster to version 1.23, then you must first install the Amazon EBS driver before updating your cluster. For more information, see [Amazon EBS CSI migration frequently asked questions](https://docs.aws.amazon.com/eks/latest/userguide/ebs-csi-migration-faq.html). If you have pods running on a version 1.22 or earlier cluster, then you must install the Amazon EBS driver before updating your cluster to version 1.23 to avoid service interruption. https://docs.aws.amazon.com/eks/latest/userguide/ebs-csi-migration-faq.html
  - Blog https://aws.amazon.com/blogs/containers/migrating-amazon-eks-clusters-from-gp2-to-gp3-ebs-volumes/
- [ ] Check AWS service limits and utilization for relevant resources
  - [ ] EC2 instances
  - [ ] EBS volumes

- [ ] Add snippets/information for commonly used provisioning tools to explain how those fit into the guidance
  - `terraform-aws-eks`/`eksctl` - how to upgrade a cluster with these tools, what will they do for the user (ensure addon versions are aligned with the Kubernetes version, the ordering of upgrade steps, etc.)
- [ ] Add test/example suite for trying out upgrades
  - Give users the ability to test out their upgrade process in a non-production environment
  - This will also double as the test suite for the tool
- [ ] Add image and chart for running `eksup` on the cluster in a continuous fashion (CronJob)
  - Can log to STDOUT or save to S3 (Athena support)
- [ ] Add support to output results in JSON and CSV formats
  - Multi-cluster scenario - all clusters emitting data back to central location to report on which clusters need components to be upgraded/modified
  - Can utilize an Athena table to agreggate and summarize data

- [ ] Converting from one resource API version to another (where possible)

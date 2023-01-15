## Notes

- Prefer topology hints over affinity for larger clusters
  - [Inter-pod affinity and anti-affinity](https://kubernetes.io/docs/concepts/scheduling-eviction/assign-pod-node/#inter-pod-affinity-and-anti-affinity)
    > Note: Inter-pod affinity and anti-affinity require substantial amount of processing which can slow down scheduling in large clusters significantly. We do not recommend using them in clusters larger than several hundred nodes.

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

## ToDo

- [ ] PDBs set on deployments/replicasets and statefulsets
- [ ] Update strategy set on deployments/replicasets and statefulsets
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
  - Can utilize an Athena table to aggregate and summarize data
- [ ] Converting from one resource API version to another (where possible)

### Nice to Have

- [ ]Progress indicator https://github.com/console-rs/indicatif

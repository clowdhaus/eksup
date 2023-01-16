## 🚧 ToDo 🚧

- [x] Version skew between control plane and data plane should adhere to skew policy; recommend they align before upgrading

### Amazon EKS

- [x] There are at least 5 free IPs in control plane subnets
  - [x] Report on number of free IPs in data plane subnets
  - [x] Report on number of free IPs used by the pods when using custom networking
- [x] Control plane is free of health issues
- [x] EKS managed node group(s) are free of health issues
- [x] EKS addon(s) are free of health issues
- [x] EKS addon version is within supported range; recommend upgrading if target Kubernetes version default addon version is newer
- [ ] EKS managed node group(s): report if the launch template version is not the latest
- [ ] Self-managed node group(s): report if the launch template version is not the latest
- [ ] Check AWS service limits and utilization for relevant resources
  - Requires premium support https://docs.aws.amazon.com/awssupport/latest/user/service-limits.html
  - [ ] EC2 instance service limits
    - `aws support describe-trusted-advisor-check-result --check-id 0Xc6LMYG8P`
  - [ ] EBS volume service limits
    - GP2 `aws support describe-trusted-advisor-check-result --check-id dH7RR0l6J9`
    - GP3 `aws support describe-trusted-advisor-check-result --check-id dH7RR0l6J3`

### Highly Available

#### Deployments/ReplicaSets/ReplicationControllers

- [ ] `.spec.replicas` set >= 3
- [ ] `.spec.minReadySeconds` set > 0 - https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#container-probes
- [ ] `.spec.strategy.type` set to `RollingUpdate` - https://kubernetes.io/docs/concepts/workloads/controllers/deployment/#rolling-update-deployment
  - [ ] `.spec.strategy.rollingUpdate.maxUnavailable` is set
  - [ ] `.spec.strategy.rollingUpdate.maxSurge` is set
- [ ] `podDisruptionBudge### Amazon Limits

- [ ] Check AWS service limits and utilization for relevant resources
  - [ ] EC2 instances
  - [ ] EBS volumes` set & at least one of `minAvailable` or `maxUnavailable` is set
- [ ] Either `podAntiAffinity` or `topologySpreadConstraints` set to avoid multiple pods from being scheduled on the same node. https://kubernetes.io/docs/concepts/configuration/assign-pod-node/
  - [ ] Prefer topology hints over affinity `Note: Inter-pod affinity and anti-affinity require substantial amount of processing which can slow down scheduling in large clusters significantly. We do not recommend using them in clusters larger than several hundred nodes.` https://kubernetes.io/docs/concepts/scheduling-eviction/assign-pod-node/#inter-pod-affinity-and-anti-affinity
- [ ] `readinessProbe` set
  - [ ] `livenessProbe` ,if set, is NOT the same as `readinessProbe`
  - [ ] `startupProbe` is set if `livenessProbe` is set

#### StatefulSets

- [ ] `.spec.replicas` set >= 3
- [ ] `.spec.minReadySeconds` set > 0 - https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#container-probes
- [ ] `.spec.updateStrategy` set to `RollingUpdate` - https://kubernetes.io/docs/concepts/workloads/controllers/statefulset/#rolling-updates
- [ ] `pod.Spec.TerminationGracePeriodSeconds` > 0 - pod.Spec.TerminationGracePeriodSeconds
- [ ] `podDisruptionBudgets` set & at least one of `minAvailable` or `maxUnavailable` is set
- [ ] Either `podAntiAffinity` or `topologySpreadConstraints` set to avoid multiple pods from being scheduled on the same node. https://kubernetes.io/docs/concepts/configuration/assign-pod-node/
  - [ ] Prefer topology hints over affinity `Note: Inter-pod affinity and anti-affinity require substantial amount of processing which can slow down scheduling in large clusters significantly. We do not recommend using them in clusters larger than several hundred nodes.` https://kubernetes.io/docs/concepts/scheduling-eviction/assign-pod-node/#inter-pod-affinity-and-anti-affinity
- [ ] `readinessProbe` set
  - [ ] `livenessProbe` , if set, is NOT the same as `readinessProbe`
  - [ ] `startupProbe` is set if `livenessProbe` is set

#### Job/CronJob

- [ ] `.spec.suspend` set to `true` before upgrading, removed after upgrade (see questions - what is the best guidance for batch workloads?)

### Kubernetes Deprecations

- [ ] Detect docker socket use (1.24+ affected) https://github.com/aws-containers/kubectl-detector-for-docker-socket
- [ ] Warn on pod security policy use (deprecated 1.21, removed 1.25) https://kubernetes.io/docs/concepts/security/pod-security-policy/
  - [ ] Advise to switch to pod security admission https://kubernetes.io/docs/concepts/security/pod-security-admission/
- [ ] Something for https://kubernetes.io/blog/2021/12/10/storage-in-tree-to-csi-migration-status-update/ ?
- [ ] The [in-tree Amazon EBS storage provisioner](https://kubernetes.io/docs/concepts/storage/volumes/#awselasticblockstore) is deprecated. If you are upgrading your cluster to version 1.23, then you must first install the Amazon EBS driver before updating your cluster. For more information, see [Amazon EBS CSI migration frequently asked questions](https://docs.aws.amazon.com/eks/latest/userguide/ebs-csi-migration-faq.html). If you have pods running on a version 1.22 or earlier cluster, then you must install the Amazon EBS driver before updating your cluster to version 1.23 to avoid service interruption. https://docs.aws.amazon.com/eks/latest/userguide/ebs-csi-migration-faq.html
  - Blog https://aws.amazon.com/blogs/containers/migrating-amazon-eks-clusters-from-gp2-to-gp3-ebs-volumes/

### Misc

- [ ] Add test/example suite for trying out upgrades
  - Give users the ability to test out their upgrade process in a non-production environment
  - This will also double as the test suite for the tool

### Future Considerations

- [ ] APIs deprecated and/or removed in the next Kubernetes version
  - For now, `pluto` or `kubent` are recommended to check for deprecated APIs
  - Add section on how those tools work, what to watch out for (asking the API Server is not trustworthy, scanning manifests directly is the most accurate)
- [ ] Add image and chart for running `eksup` on the cluster in a continuous fashion (CronJob)
  - Can log to STDOUT or save to S3 (Athena support)
- [ ] Add support to output results in JSON and CSV formats
  - Multi-cluster scenario - all clusters emitting data back to central location to report on which clusters need components to be upgraded/modified
  - Can utilize an Athena table to aggregate and summarize data
- [ ] Configuration file to allow users more control over what checks they want to opt in/out of, the values of those checks, etc.
- [ ] Progress indicator https://github.com/console-rs/indicatif
- [ ] Ability to convert from one resource API version to another (where possible)
- [ ] Add snippets/information for commonly used provisioning tools to explain how those fit into the guidance
  - `terraform-aws-eks`/`eksctl` - how to upgrade a cluster with these tools, what will they do for the user (ensure addon versions are aligned with the Kubernetes version, the ordering of upgrade steps, etc.)
- [ ] Configure output levels
    1. `--quiet` - suppress all output
    2. (default, no flags) - show failed checks on hard requirements
    3. `--warn` - in addition to failed, show warnings (low number of IPs available for nodes/pods, addon version older than current default, etc.)
    4. `--info` - in addition to failed and warnings, show informational notices (number of IPs available for nodes/pods, addon version relative to current default and latest, etc.)

## Notes

- Prefer topology hints over affinity for larger clusters
  - [Inter-pod affinity and anti-affinity](https://kubernetes.io/docs/concepts/scheduling-eviction/assign-pod-node/#inter-pod-affinity-and-anti-affinity)
    > Note: Inter-pod affinity and anti-affinity require substantial amount of processing which can slow down scheduling in large clusters significantly. We do not recommend using them in clusters larger than several hundred nodes.

## Questions

- What is the guidance for batch workloads?
  - Recommend creating a maintenance window where workloads should avoid being scheduled?
  - `JobFailurePolicy` coming in in 1.26 https://kubernetes.io/docs/concepts/workloads/controllers/job/#pod-failure-policy
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

## Kubernetes Future Features

Relevant features that are coming in future releases of Kubernetes. A feature is "relevant" in in this context if it is something that would be checked and reported on by `eksup` to aid in upgrades:
- `.spec.updateStrategy.rollingUpdate.maxUnavailable` for StatefulSets  [`Kubernetes v1.24 [alpha]`](https://kubernetes.io/docs/concepts/workloads/controllers/statefulset/#maximum-unavailable-pods)
  - Recommend that a value is set on all StatefulSets
- `PodDisruptionCondition` for PodDisruptionBudgets [`Kubernetes v1.26 [beta]`](https://kubernetes.io/docs/concepts/workloads/pods/disruptions/#pod-disruption-conditions)
  - See recommendation below for `podFailurePolicy`
- `.spec.podFailurePolicy` for Jobs/CronJobs [`Kubernetes v1.26 [beta]`](https://kubernetes.io/docs/concepts/workloads/controllers/job/#pod-failure-policy)
  - Recommend to `Ignore` conditions caused by preemption, API-initiated eviction, or taint-based eviction so that upgrade type evictions do not count against `.spec.backoffLimit` and the jobs will be re-tried. Note - `.spec.restartPolicy` will need to be set to `Never` and `PodDisruptionCondition` must be set for PodDisruptionBudgets

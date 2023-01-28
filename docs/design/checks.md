# Checks

## Amazon

Checks that are not specific to Amazon EKS or Kubernetes

#### AWS001
🚧 _Not yet implemented_

Report on number of free IPs in data plane subnets

#### AWS002

Report on number of free IPs used by the pods when using VPC CNI custom networking

#### AWS003
🚧 _Not yet implemented_

EC2 instance service limits

#### AWS004
🚧 _Not yet implemented_

EBS GP2 volume service limits

#### AWS005
🚧 _Not yet implemented_

EBS GP3 volume service limits

## Amazon EKS

|  Check   | Implemented | Description                                                                                                                  |
| :------: | :---------: | :--------------------------------------------------------------------------------------------------------------------------- |
| `EKS001` |     ✅      | There are at least 5 free IPs in control plane subnets                                                                       |
| `EKS002` |     ✅      | Control plane is free of health issues                                                                                       |
| `EKS003` |     ✅      | EKS managed node group(s) are free of health issues                                                                          |
| `EKS004` |     ✅      | EKS addon(s) are free of health issues                                                                                       |
| `EKS005` |     ✅      | EKS addon version is within supported range; recommend upgrading if target Kubernetes version default addon version is newer |
| `EKS006` |     ✅      | EKS managed node group(s): report if the launch template version is not the latest                                           |
| `EKS007` |     ✅      | Self-managed node group(s): report if the launch template version is not the latest                                          |

## Kubernetes

| Check | Description |
| :---: | :---------- |
| `K8S001` | Version skew between control plane and data plane should adhere to skew policy |
| `K8S002` | `.spec.replicas` set >= 3 |
| `K8S003` | `.spec.minReadySeconds` set > 0 - https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#container-probes |
| `K8S004` | Rolling update strategy is used |
| `K8S005` | `podDisruptionBudgets` set & at least one of `minAvailable` or `maxUnavailable` is set |
| `K8S006` | Either `.spec.affinity.podAntiAffinity` or `.spec.topologySpreadConstraints` set to avoid multiple pods from being scheduled on the same node. https://kubernetes.io/docs/concepts/configuration/assign-pod-node/ |
| `K8S007` | `.spec.containers[*].readinessProbe` set |
| `K8S008` |  `pod.Spec.TerminationGracePeriodSeconds` > 0 - The StatefulSet should not specify a pod.Spec.TerminationGracePeriodSeconds of 0 https://kubernetes.io/docs/concepts/workloads/controllers/statefulset/#deployment-and-scaling-guarantees |
| `K8S009` |  Detect docker socket use (1.24+ affected) https://github.com/aws-containers/kubectl-detector-for-docker-socket |
| `K8S010` | `.spec.template.spec.dnsPolicy` set to `ClusterFirst` |
| `K8S011` | `.spec.template.spec.terminationGracePeriodSeconds` set to `30` |

### Kubernetes Highly Available

|   Check    | Deployment | ReplicaSet | ReplicationController | StatefulSet | Job | CronJob | Daemonset |
| :--------: | :--------: | :--------: | :-------------------: | :---------: | :-: | :-----: | :-------: |
| [`K8S001` |     -      |     -      |           -           |      -      |  -  |    -    |     -     |
| [`K8S002` |     ✅     |     ✅     |          ✅           |     ✅      | ❌  |   ❌    |    ❌     |
| [`K8S003` |     ✅     |     ✅     |          ✅           |     ✅      | ❌  |   ❌    |    ❌     |
| [`K8S004` |     ✅     |     ✅     |          ❌           |     ✅      | ❌  |   ❌    |    ❌     |
| [`K8S005` |     ✅     |     ✅     |          ✅           |     ✅      | ❌  |   ❌    |    ❌     |
| [`K8S006` |     ✅     |     ✅     |          ✅           |     ✅      | ❌  |   ❌    |    ❌     |
| [`K8S007` |     ✅     |     ✅     |          ✅           |     ✅      | ❌  |   ❌    |    ❌     |
| [`K8S008` |     ❌     |     ❌     |          ❌           |     ✅      | ❌  |   ❌    |    ❌     |
| [`K8S009` |     ✅     |     ✅     |          ✅           |     ✅      | ✅  |   ✅    |    ✅     |
| [`K8S010` |     -      |     -      |           -           |      -      |  -  |    -    |     -     |
| [`K8S011` |     -      |     -      |           -           |      -      |  -  |    -    |     -     |

- [ ] [`K8S002`] `.spec.replicas` set >= 3
- [ ] [`K8S003`] `.spec.minReadySeconds` set > 0 - https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#container-probes
- [ ] [`K8S004`] rolling update strategy is used
  - `.spec.strategy.type` != `Recreate` - https://kubernetes.io/docs/concepts/workloads/controllers/deployment/#rolling-update-deployment
    - (Deployment, ReplicaSet) - excludes ReplicationController which recommends blue/green upgrade
    - [ ] `.spec.strategy.rollingUpdate.maxUnavailable` is set (Recommended)
    - [ ] `.spec.strategy.rollingUpdate.maxSurge` is set (Recommended)
  - [ ] `.spec.updateStrategy.type` != `OnDelete` - https://kubernetes.io/docs/concepts/workloads/controllers/statefulset/#rolling-updates
    - (StatefulSet)
- [ ] [`K8S005`] `podDisruptionBudgets` set & at least one of `minAvailable` or `maxUnavailable` is set
- [ ] [`K8S006`] Either `.spec.affinity.podAntiAffinity` or `.spec.topologySpreadConstraints` set to avoid multiple pods from being scheduled on the same node. https://kubernetes.io/docs/concepts/configuration/assign-pod-node/
  - [ ] Prefer topology hints over affinity `Note: Inter-pod affinity and anti-affinity require substantial amount of processing which can slow down scheduling in large clusters significantly. We do not recommend using them in clusters larger than several hundred nodes.` https://kubernetes.io/docs/concepts/scheduling-eviction/assign-pod-node/#inter-pod-affinity-and-anti-affinity
- [ ] [`K8S007`] `.spec.containers[*].readinessProbe` set
  - [ ] `.spec.containers[*].livenessProbe` , if set, is NOT the same as `.spec.containers[*].readinessProbe`
  - [ ] `.spec.containers[*].startupProbe` is set if `.spec.containers[*].livenessProbe` is set
- [ ] [`K8S008`] `pod.Spec.TerminationGracePeriodSeconds` > 0 - The StatefulSet should not specify a pod.Spec.TerminationGracePeriodSeconds of 0 https://kubernetes.io/docs/concepts/workloads/controllers/statefulset/#deployment-and-scaling-guarantees
  - (StatefulSet)

#### Job/CronJob

- [ ] [`K8S---`] `.spec.suspend` set to `true` before upgrading, removed after upgrade (see questions - what is the best guidance for batch workloads?)

### Kubernetes Deprecations

Note: the Kubernetes version these apply to will need to be taken into consideration to avoid telling users about checks that do not apply to their version.

- [ ] [`K8S009`] Detect docker socket use (1.24+ affected) https://github.com/aws-containers/kubectl-detector-for-docker-socket
- [ ] [`K8S010`] Warn on pod security policy use (deprecated 1.21, removed 1.25) https://kubernetes.io/docs/concepts/security/pod-security-policy/
  - [ ] Advise to switch to pod security admission https://kubernetes.io/docs/concepts/security/pod-security-admission/
- [ ] [`K8S011`] In-tree to CSI migration https://kubernetes.io/blog/2021/12/10/storage-in-tree-to-csi-migration-status-update/ ?
  - [ ] The [in-tree Amazon EBS storage provisioner](https://kubernetes.io/docs/concepts/storage/volumes/#awselasticblockstore) is deprecated. If you are upgrading your cluster to version 1.23, then you must first install the Amazon EBS driver before updating your cluster. For more information, see [Amazon EBS CSI migration frequently asked questions](https://docs.aws.amazon.com/eks/latest/userguide/ebs-csi-migration-faq.html). If you have pods running on a version 1.22 or earlier cluster, then you must install the Amazon EBS driver before updating your cluster to version 1.23 to avoid service interruption. https://docs.aws.amazon.com/eks/latest/userguide/ebs-csi-migration-faq.html
  - Blog https://aws.amazon.com/blogs/containers/migrating-amazon-eks-clusters-from-gp2-to-gp3-ebs-volumes/

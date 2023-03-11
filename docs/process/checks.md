# Checks

If a check fails, it is reported as a finding. Each check will have a remediation type - either recommended or required. A recommended remediation is one that is recommended to be performed, but is not required to be performed.

- ⚠️ Recommended: A finding that users are encouraged to evaluate the recommendation and determine if it is applicable and whether or not to act upon that recommendation. Not remediating the finding does not prevent the upgrade from occurring.
- ❌ Required: A finding that requires remediation prior to upgrading to be able to perform the upgrade and avoid downtime or disruption

See the [symbol table](https://clowdhaus.github.io/eksup/process/#symbol-table) for further details on the symbols used throughout the documentation.

## Amazon

Checks that are not specific to Amazon EKS or Kubernetes

#### AWS001

!!! info "🚧 _Not yet implemented_"

**⚠️ Remediation recommended**

There is a sufficient quantity of IPs available for the nodes to support the upgrade.

If custom networking is enabled, the results represent the number of IPs available in the subnets used by the EC2 instances. Otherwise, the results represent the number of IPs available in the subnets used by both the EC2 instances and the pods.

#### AWS002

**⚠️ Remediation recommended**

There is a sufficient quantity of IPs available for the **pods** to support the upgrade.

This check is used when custom networking is enabled since the IPs used by pods are coming from subnets different from those used by the EC2 instances themselves.

#### AWS003
!!! info "🚧 _Not yet implemented_"

EC2 instance service limits

#### AWS004
!!! info "🚧 _Not yet implemented_"

EBS GP2 volume service limits

#### AWS005
!!! info "🚧 _Not yet implemented_"

EBS GP3 volume service limits

---

## Amazon EKS

Checks that are specific to Amazon EKS

#### EKS001

**❌ Remediation required**

There are at least 2 subnets in different availability zones, each with at least 5 available IPs for the control plane to upgrade.

#### EKS002

**❌ Remediation required**

Control plane does not have any reported health issues.

#### EKS003

**❌ Remediation required**

EKS managed nodegroup does not have any reported health issues.

This does not include self-managed nodegroups or Fargate profiles; those are not currently supported by the AWS API to report health issues.

#### EKS004

**❌ Remediation required**

EKS addon does not have any reported health issues.

#### EKS005

**❌ Remediation required**

EKS addon version is within the supported range.

The addon must be updated to a version that is supported by the target Kubernetes version prior to upgrading.

**⚠️ Remediation recommended**

The target Kubernetes version default addon version is newer than the current addon version.

For example, if the default addon version of CoreDNS for Kubernetes `v1.24` is `v1.8.7-eksbuild.3` and the current addon version is `v1.8.4-eksbuild.2`, while the current version is supported on Kubernetes `v1.24`, its recommended to update the addon to `v1.8.7-eksbuild.3` during the upgrade.

#### EKS006

**⚠️ Remediation recommended**

EKS managed nodegroup are using the latest launch template version and there are no pending updates for the nodegroup.

Users are encourage to evaluate if remediation is warranted or not and whether to update to the latest launch template version prior to upgrading. If there are pending updates, this could potentially introduce additional changes to the nodegroup during the upgrade.

<!-- TODO - add the CLI command to diff the launch template versions
diff <(aws ec2 describe-launch-template-versions A ...) <(aws ec2 describe-launch-template-versions B ...) -->
<!-- TODO - consider diffing the templates and reporting the differences in the reported output -->

#### EKS007

**⚠️ Remediation recommended**

Self-managed nodegroup are using the latest launch template version and there are no pending updates for the nodegroup.

Users are encourage to evaluate if remediation is warranted or not and whether to update to the latest launch template version prior to upgrading. If there are pending updates, this could potentially introduce additional changes to the nodegroup during the upgrade.

<!-- TODO - add the CLI command to diff the launch template versions
diff <(aws ec2 describe-launch-template-versions A ...) <(aws ec2 describe-launch-template-versions B ...) -->
<!-- TODO - consider diffing the templates and reporting the differences in the reported output -->

---

## Kubernetes

Checks that are specific to Kubernetes, regardless of the underlying platform provider.

Table below shows the checks that are applicable, or not, to the respective Kubernetes resource.

|  Check   | Deployment | ReplicaSet | ReplicationController | StatefulSet | Job | CronJob | Daemonset |
| :------: | :--------: | :--------: | :-------------------: | :---------: | :-: | :-----: | :-------: |
| `K8S001` |    󠀭➖     |     ➖     |          ➖           |     ➖      | ➖  |   ➖    |    ➖     |
| `K8S002` |     ✅     |     ✅     |          ✅           |     ✅      | ❌  |   ❌    |    ❌     |
| `K8S003` |     ✅     |     ✅     |          ✅           |     ✅      | ❌  |   ❌    |    ❌     |
| `K8S004` |     ✅     |     ✅     |          ❌           |     ✅      | ❌  |   ❌    |    ❌     |
| `K8S005` |     ✅     |     ✅     |          ✅           |     ✅      | ❌  |   ❌    |    ❌     |
| `K8S006` |     ✅     |     ✅     |          ✅           |     ✅      | ❌  |   ❌    |    ❌     |
| `K8S007` |     ✅     |     ✅     |          ✅           |     ✅      | ❌  |   ❌    |    ❌     |
| `K8S008` |     ❌     |     ❌     |          ❌           |     ✅      | ❌  |   ❌    |    ❌     |
| `K8S009` |     ✅     |     ✅     |          ✅           |     ✅      | ✅  |   ✅    |    ✅     |
| `K8S010` |     ➖     |     ➖     |          ➖           |     ➖      | ➖  |   ➖    |    ➖     |
| `K8S011` |     ➖     |     ➖     |          ➖           |     ➖      | ➖  |   ➖    |    ➖     |

#### K8S001

**❌ Remediation required**

The version skew between the control plane (API Server) and the data plane (kubelet) violates the Kubernetes version skew policy, or will violate the version skew policy after the control plane has been upgraded.

The data plane nodes must be upgraded to at least within 1 minor version of the control plane version in order to stay within the version skew policy through the upgrade; it is recommended to upgrade the data plane nodes to the same version as the control plane.

**⚠️ Remediation recommended**

There is a version skew between the control plane (API Server) and the data plane (kubelet).

While Kubernetes does support a version skew of n-2 between the API Server and kubelet, it is recommended to upgrade the data plane nodes to the same version as the control plane.

[Kubernetes version skew policy](https://kubernetes.io/releases/version-skew-policy/#supported-version-skew)

#### K8S002

**❌ Remediation required**

There are at least 3 replicas specified for the resource.

```yaml

---
spec:
  replicas: 3 # >= 3
```

Multiple replicas, along with the use of `PodDisruptionBudget`, are required to ensure high availability during the upgrade.

[EKS Best Practices - Reliability](https://aws.github.io/aws-eks-best-practices/reliability/docs/application/#run-multiple-replicas)

#### K8S003

**❌ Remediation required**

`minReadySeconds` has been set to a value greater than 0 seconds for `StatefulSet`

You can read more about why this is necessary for `StatefulSet` [here](https://kubernetes.io/blog/2021/08/27/minreadyseconds-statefulsets/)

**⚠️ Remediation recommended**

`minReadySeconds` has been set to a value greater than 0 seconds for `Deployment`, `ReplicaSet`, `ReplicationController`

#### K8S004

!!! info "🚧 _Not yet implemented_"

**❌ Remediation required**

At least one `podDisruptionBudget` covers the workload, and at least one of `minAvailable` or `maxUnavailable` is set

The Kubernetes eviction API is the preferred method for draining nodes for replacement during an upgrade. The eviction API respects `PodDisruptionBudget` and will not evict pods that would violate the `PodDisruptionBudget` to ensure application availability, when specified.

#### K8S005

**❌ Remediation required**

Either `.spec.affinity.podAntiAffinity` or `.spec.topologySpreadConstraints` is set to avoid multiple pods from the same workload from being scheduled on the same node.

`topologySpreadConstraints` are preferred over affinity, especially for larger clusters:

  - [Inter-pod affinity and anti-affinity](https://kubernetes.io/docs/concepts/scheduling-eviction/assign-pod-node/#inter-pod-affinity-and-anti-affinity)

    > Note: Inter-pod affinity and anti-affinity require substantial amount of processing which can slow down scheduling in large clusters significantly. We do not recommend using them in clusters larger than several hundred nodes.

[Types of inter-pod affinity and anti-affinity](https://kubernetes.io/docs/concepts/scheduling-eviction/assign-pod-node/#types-of-inter-pod-affinity-and-anti-affinity)

[Pod Topology Spread Constraints](https://kubernetes.io/docs/concepts/scheduling-eviction/topology-spread-constraints/)

#### K8S006

**❌ Remediation required**

A `readinessProbe` must be set to ensure traffic is not routed to pods before they are ready following their re-deployment from a node replacement.

#### K8S007

**❌ Remediation required**

The `StatefulSet` should not specify a `TerminationGracePeriodSeconds` of 0

  - [Deployment and Scaling Guarantees](https://kubernetes.io/docs/concepts/workloads/controllers/statefulset/#deployment-and-scaling-guarantees)

    > The StatefulSet should not specify a pod.Spec.TerminationGracePeriodSeconds of 0. This practice is unsafe and strongly discouraged. For further explanation, please refer to force deleting StatefulSet Pods.

[Force Delete StatefulSet Pods](https://kubernetes.io/docs/tasks/run-application/force-delete-stateful-set-pod/)

#### K8S008

Pod volumes should not mount the `docker.sock` file with the removal of the Dockershim starting in Kubernetes `v1.24`

**❌ Remediation required**

For clusters on Kubernetes `v1.23`

**⚠️ Remediation recommended**

For clusters on Kubernetes <`v1.22`

[Dockershim Removal FAQ](https://kubernetes.io/blog/2022/02/17/dockershim-faq/)

[Detector for Docker Socket (DDS)](https://github.com/aws-containers/kubectl-detector-for-docker-socket)

#### K8S009

The pod security policy resource has been removed started in Kubernetes `v1.25`

**❌ Remediation required**

For clusters on Kubernetes `v1.24`

**⚠️ Remediation recommended**

For clusters on Kubernetes <`v1.23`

[Migrate from PodSecurityPolicy to the Built-In PodSecurity Admission Controller](https://kubernetes.io/docs/tasks/configure-pod-container/migrate-from-psp/)

[PodSecurityPolicy Deprecation: Past, Present, and Future](https://kubernetes.io/blog/2021/04/06/podsecuritypolicy-deprecation-past-present-and-future/)

#### K8S010

!!! info "🚧 _Not yet implemented_"

The [in-tree Amazon EBS storage provisioner](https://kubernetes.io/docs/concepts/storage/volumes/#awselasticblockstore) is deprecated. If you are upgrading your cluster to version `v1.23`, then you must first install the Amazon EBS driver before updating your cluster. For more information, see [Amazon EBS CSI migration frequently asked questions](https://docs.aws.amazon.com/eks/latest/userguide/ebs-csi-migration-faq.html).

**❌ Remediation required**

For clusters on Kubernetes `v1.22`

**⚠️ Remediation recommended**

For clusters on Kubernetes <`v1.21`

[Amazon EBS CSI migration frequently asked questions](https://docs.aws.amazon.com/eks/latest/userguide/ebs-csi-migration-faq.html)

[Kubernetes In-Tree to CSI Volume Migration Status Update](https://kubernetes.io/blog/2021/12/10/storage-in-tree-to-csi-migration-status-update/)

#### K8S011

**❌ Remediation required**

`kube-proxy` on an Amazon EKS cluster has the same [compatibility and skew policy as Kubernetes](https://kubernetes.io/releases/version-skew-policy/#kube-proxy)

- It must be the same minor version as kubelet on your Amazon EC2 nodes
- It cannot be newer than the minor version of your cluster's control plane
- Its version on your Amazon EC2 nodes can't be more than two minor versions older than your control plane. For example, if your control plane is running Kubernetes `1.25`, then the kube-proxy minor version cannot be older than `1.23`

If you recently updated your cluster to a new Kubernetes minor version, then update your Amazon EC2 nodes (i.e. - `kubelet`) to the same minor version before updating `kube-proxy` to the same minor version as your nodes. The order of operations during an upgrade are as follows:

    1. Update the control plane to the new Kubernetes minor version
    2. Update the nodes, which updates `kubelet`, to the new Kubernetes minor version
    3. Update `kube-proxy` to the new Kubernetes minor version

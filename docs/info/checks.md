# Checks

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14](https://www.rfc-editor.org/info/bcp14) [[RFC 2119](https://www.rfc-editor.org/rfc/rfc2119)] [[RFC 8174](https://www.rfc-editor.org/rfc/rfc8174)] when, and only when, they appear in all capitals, as shown here.

If a check fails, it is reported as a finding. Each check will have a remediation type - either recommended or required. A recommended remediation is one that is recommended to be performed, but is not required to be performed.

- ⚠️ Recommended: A finding that users are encouraged to evaluate the recommendation and determine if it is applicable and whether or not to act upon that recommendation. Not remediating the finding does not prevent the upgrade from occurring.
- ❌ Required: A finding that requires remediation prior to upgrading to be able to perform the upgrade and avoid downtime or disruption

See the [symbol table](https://clowdhaus.github.io/eksup/#symbol-table) for further details on the symbols used throughout the documentation.

## Summary

<!-- BEGIN GENERATED CHECKS TABLE -->

| Code | Description | Status | Applicable Versions |
| :--- | :---------- | :----- | :------------------ |
| `AWS001` | Insufficient available subnet IPs for nodes | Active | All versions |
| `AWS002` | Insufficient available subnet IPs for pods (custom networking) | Active | All versions |
| `AWS003` | Insufficient EC2 service limits | Active | All versions |
| `AWS004` | Insufficient EBS GP2 service limits | Active | All versions |
| `AWS005` | Insufficient EBS GP3 service limits | Active | All versions |
| `EKS001` | Insufficient available subnet IPs for control plane ENIs | Active | All versions |
| `EKS002` | Health issue(s) reported by the EKS control plane | Active | All versions |
| `EKS003` | Health issue(s) reported by the EKS managed node group | Active | All versions |
| `EKS004` | Health issue(s) reported by the EKS addon | Active | All versions |
| `EKS005` | EKS addon incompatible with targeted Kubernetes version | Active | All versions |
| `EKS006` | EKS managed node group has pending launch template update(s) | Active | All versions |
| `EKS007` | Self-managed node group has pending launch template update(s) | Active | All versions |
| `EKS008` | AL2 AMI deprecation (deprecated in 1.32, removed in 1.33+) | Active | 1.32+ |
| `EKS009` | EKS upgrade readiness insight | Active | All versions |
| `EKS010` | EKS cluster misconfiguration insight | Active | All versions |
| `K8S001` | Kubernetes version skew between control plane and node | Active | All versions |
| `K8S002` | Insufficient number of .spec.replicas (configurable) | Active | All versions |
| `K8S003` | Insufficient .spec.minReadySeconds | Active | All versions |
| `K8S004` | Missing PodDisruptionBudget | Active | All versions |
| `K8S005` | Pod distribution settings put availability at risk | Active | All versions |
| `K8S006` | Missing readinessProbe on containers | Active | All versions |
| `K8S007` | TerminationGracePeriodSeconds is set to zero | Active | All versions |
| `K8S008` | Mounts docker.sock or dockershim.sock | Active | All versions |
| `K8S009` | Pod security policies present (removed in 1.25) | Retired | Up to 1.24 |
| `K8S010` | EBS CSI driver not installed | Retired | Up to 1.24 |
| `K8S011` | kube-proxy version skew with kubelet | Active | All versions |
| `K8S012` | kube-proxy IPVS mode deprecated (1.35+, removed 1.36) | Active | 1.35+ |
| `K8S013` | Ingress NGINX controller retirement (1.35+) | Active | 1.35+ |

<!-- END GENERATED CHECKS TABLE -->

## Amazon

Checks that are not specific to Amazon EKS or Kubernetes

#### AWS001

**⚠️ Remediation recommended**

There MUST be a sufficient quantity of IPs available for the nodes to support the upgrade.

If custom networking is enabled, the results represent the number of IPs available in the subnets used by the EC2 instances. Otherwise, the results represent the number of IPs available in the subnets used by both the EC2 instances and the pods.

#### AWS002

**⚠️ Remediation recommended**

There MUST be a sufficient quantity of IPs available for the **pods** to support the upgrade.

This check is used when custom networking is enabled since the IPs used by pods are coming from subnets different from those used by the EC2 instances themselves.

#### AWS003

**⚠️ Remediation recommended**

There MUST be sufficient EC2 instance service limits to support the upgrade. During an upgrade, additional instances may be launched temporarily (e.g., by managed node groups or auto-scaling groups), and hitting service limits could prevent new nodes from joining the cluster.

#### AWS004

**⚠️ Remediation recommended**

There MUST be sufficient EBS GP2 volume service limits to support the upgrade. Persistent volumes backed by GP2 may need to be re-attached to new nodes during the upgrade process.

#### AWS005

**⚠️ Remediation recommended**

There MUST be sufficient EBS GP3 volume service limits to support the upgrade. Persistent volumes backed by GP3 may need to be re-attached to new nodes during the upgrade process.

---

## Amazon EKS

Checks that are specific to Amazon EKS

#### EKS001

**❌ Remediation required**

There MUST be at least 2 subnets in different availability zones, each with at least 5 available IPs for the control plane to upgrade.

#### EKS002

**❌ Remediation required**

Control plane MUST NOT have any reported health issues.

#### EKS003

**❌ Remediation required**

EKS managed nodegroup MUST NOT have any reported health issues.

This does not include self-managed nodegroups or Fargate profiles; those are not currently supported by the AWS API to report health issues.

#### EKS004

**❌ Remediation required**

EKS addon MUST NOT have any reported health issues.

#### EKS005

**❌ Remediation required**

EKS addon version MUST be within the supported range.

The addon MUST be updated to a version that is supported by the target Kubernetes version prior to upgrading.

**⚠️ Remediation recommended**

The target Kubernetes version default addon version is newer than the current addon version.

For example, if the default addon version of CoreDNS for Kubernetes `v1.24` is `v1.8.7-eksbuild.3` and the current addon version is `v1.8.4-eksbuild.2`, while the current version is supported on Kubernetes `v1.24`, it is RECOMMENDED to update the addon to `v1.8.7-eksbuild.3` during the upgrade.

#### EKS006

**⚠️ Remediation recommended**

EKS managed nodegroups SHOULD use the latest launch template version and there SHOULD NOT be any pending updates for the nodegroup.

Users are encouraged to evaluate if remediation is warranted or not and whether to update to the latest launch template version prior to upgrading. If there are pending updates, this could potentially introduce additional changes to the nodegroup during the upgrade.

#### EKS007

**⚠️ Remediation recommended**

Self-managed nodegroups SHOULD use the latest launch template version and there SHOULD NOT be any pending updates for the nodegroup.

Users are encouraged to evaluate if remediation is warranted or not and whether to update to the latest launch template version prior to upgrading. If there are pending updates, this could potentially introduce additional changes to the nodegroup during the upgrade.

#### EKS008

EKS managed nodegroups MUST NOT use AL2 (Amazon Linux 2) AMI types. AL2 AMIs are deprecated starting in Kubernetes 1.32 and are no longer supported in 1.33+. Users MUST migrate to AL2023 or Bottlerocket AMI types.

**❌ Remediation required**

For clusters upgrading to Kubernetes `v1.33` or later — AL2 AMI types MUST NOT be used as they are no longer supported.

**⚠️ Remediation recommended**

For clusters upgrading to Kubernetes `v1.32` — AL2 AMI types are deprecated and migration SHOULD be completed before they become unsupported.

[Amazon Linux 2 end of standard support](https://docs.aws.amazon.com/linux/al2/ug/eol-faq.html)

#### EKS009

**⚠️ Remediation recommended**

EKS upgrade readiness insights are reported by the EKS API to help identify potential issues before upgrading. These insights are specific to upgrade compatibility and cover deprecated APIs, unsupported configurations, and other upgrade blockers detected by the EKS service.

Users SHOULD review and address any upgrade readiness insights before proceeding with the cluster upgrade.

[Amazon EKS cluster insights](https://docs.aws.amazon.com/eks/latest/userguide/cluster-insights.html)

#### EKS010

**⚠️ Remediation recommended**

EKS cluster misconfiguration insights are reported by the EKS API to identify configuration issues that may affect cluster health or functionality. Unlike upgrade readiness insights (EKS009), these are not specific to a particular upgrade but reflect general best practices and misconfigurations.

Users SHOULD review and address any cluster misconfiguration insights to maintain cluster health.

[Amazon EKS cluster insights](https://docs.aws.amazon.com/eks/latest/userguide/cluster-insights.html)

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
| `K8S011` |     ➖     |     ➖     |          ➖           |     ➖      | ➖  |   ➖    |    ➖     |
| `K8S012` |     ➖     |     ➖     |          ➖           |     ➖      | ➖  |   ➖    |    ➖     |
| `K8S013` |     ✅     |     ❌     |          ❌           |     ❌      | ❌  |   ❌    |    ✅     |

#### K8S001

**❌ Remediation required**

The version skew between the control plane (API Server) and the data plane (kubelet) MUST NOT violate the Kubernetes version skew policy, either currently or after the control plane has been upgraded.

The data plane nodes MUST be upgraded to at least within 3 minor versions of the control plane version in order to stay within the version skew policy through the upgrade; it is RECOMMENDED to upgrade the data plane nodes to the same version as the control plane.

**⚠️ Remediation recommended**

There is a version skew between the control plane (API Server) and the data plane (kubelet).

While Kubernetes does support a version skew of n-3 between the API Server and kubelet, the data plane nodes SHOULD be upgraded to the same version as the control plane.

[Kubernetes version skew policy](https://kubernetes.io/releases/version-skew-policy/#supported-version-skew)

#### K8S002

**❌ Remediation required**

There MUST be at least the configured minimum number of replicas specified for the resource. The default minimum is 2 replicas, which can be customized via `.eksup.yaml`.

```yaml
---
spec:
  replicas: 2 # >= configured minimum (default: 2)
```

Multiple replicas, along with the use of `PodDisruptionBudget`, are REQUIRED to ensure high availability during the upgrade.

The minimum replica threshold, ignored resources, and per-resource overrides can be configured:

```yaml
checks:
  K8S002:
    min_replicas: 3  # Global minimum replica threshold (default: 2)
    ignore:          # Resources to skip entirely
      - name: metrics-server
        namespace: kube-system
    overrides:       # Per-resource threshold overrides
      - name: critical-app
        namespace: production
        min_replicas: 5
```

[EKS Best Practices - Reliability](https://aws.github.io/aws-eks-best-practices/reliability/docs/application/#run-multiple-replicas)

#### K8S003

**❌ Remediation required**

`minReadySeconds` MUST be set to a value greater than 0 seconds for `StatefulSet`

You can read more about why this is necessary for `StatefulSet` [here](https://kubernetes.io/blog/2021/08/27/minreadyseconds-statefulsets/)

**⚠️ Remediation recommended**

`minReadySeconds` SHOULD be set to a value greater than 0 seconds for `Deployment`, `ReplicaSet`, `ReplicationController`

#### K8S004

**❌ Remediation required**

At least one `podDisruptionBudget` MUST cover the workload, and at least one of `minAvailable` or `maxUnavailable` MUST be set.

The Kubernetes eviction API is the preferred method for draining nodes for replacement during an upgrade. The eviction API respects `PodDisruptionBudget` and will not evict pods that would violate the `PodDisruptionBudget` to ensure application availability, when specified.

Resources can be excluded from this check via `.eksup.yaml`:

```yaml
checks:
  K8S004:
    ignore:          # Resources to skip PDB check
      - name: singleton-worker
        namespace: batch
```

#### K8S005

**❌ Remediation required**

Either `.spec.affinity.podAntiAffinity` or `.spec.topologySpreadConstraints` MUST be set to avoid multiple pods from the same workload from being scheduled on the same node.

`topologySpreadConstraints` are preferred over affinity, especially for larger clusters:

  - [Inter-pod affinity and anti-affinity](https://kubernetes.io/docs/concepts/scheduling-eviction/assign-pod-node/#inter-pod-affinity-and-anti-affinity)

    > Note: Inter-pod affinity and anti-affinity require substantial amount of processing which can slow down scheduling in large clusters significantly. We do not recommend using them in clusters larger than several hundred nodes.

[Types of inter-pod affinity and anti-affinity](https://kubernetes.io/docs/concepts/scheduling-eviction/assign-pod-node/#types-of-inter-pod-affinity-and-anti-affinity)

[Pod Topology Spread Constraints](https://kubernetes.io/docs/concepts/scheduling-eviction/topology-spread-constraints/)

#### K8S006

**❌ Remediation required**

A `readinessProbe` MUST be set to ensure traffic is not routed to pods before they are ready following their re-deployment from a node replacement.

#### K8S007

**❌ Remediation required**

The `StatefulSet` MUST NOT specify a `TerminationGracePeriodSeconds` of 0.

  - [Deployment and Scaling Guarantees](https://kubernetes.io/docs/concepts/workloads/controllers/statefulset/#deployment-and-scaling-guarantees)

    > The StatefulSet should not specify a pod.Spec.TerminationGracePeriodSeconds of 0. This practice is unsafe and strongly discouraged. For further explanation, please refer to force deleting StatefulSet Pods.

[Force Delete StatefulSet Pods](https://kubernetes.io/docs/tasks/run-application/force-delete-stateful-set-pod/)

#### K8S008

Pod volumes MUST NOT mount the `docker.sock` file with the removal of the Dockershim starting in Kubernetes `v1.24`.

**❌ Remediation required**

For clusters on Kubernetes `v1.23` — Pod volumes MUST NOT mount the `docker.sock` file.

**⚠️ Remediation recommended**

For clusters on Kubernetes <`v1.22` — Pod volumes SHOULD NOT mount the `docker.sock` file.

[Dockershim Removal FAQ](https://kubernetes.io/blog/2022/02/17/dockershim-faq/)

[Detector for Docker Socket (DDS)](https://github.com/aws-containers/kubectl-detector-for-docker-socket)

#### K8S011

**❌ Remediation required**

`kube-proxy` on an Amazon EKS cluster MUST follow the same [compatibility and skew policy as Kubernetes](https://kubernetes.io/releases/version-skew-policy/#kube-proxy):

- It MUST NOT be newer than the minor version of your cluster's control plane.
- Its version MUST NOT be more than three minor versions older than your control plane (API server). For example, if your control plane is running Kubernetes `1.25`, then the kube-proxy minor version MUST NOT be older than `1.22`.

If you recently updated your cluster to a new Kubernetes minor version, then update your Amazon EC2 nodes (i.e. - `kubelet`) to the same minor version before updating `kube-proxy` to the same minor version as your nodes. The order of operations during an upgrade are as follows:

    1. Update the control plane to the new Kubernetes minor version
    2. Update the nodes, which updates `kubelet`, to the new Kubernetes minor version
    3. Update `kube-proxy` to the new Kubernetes minor version

#### K8S012

`kube-proxy` IPVS proxy mode is deprecated starting in Kubernetes `v1.35` and will be removed in `v1.36`. Clusters using IPVS mode MUST migrate to `iptables` or `nftables` proxy mode.

**❌ Remediation required**

For clusters upgrading to Kubernetes `v1.36` or later — IPVS proxy mode MUST NOT be used as it is removed.

**⚠️ Remediation recommended**

For clusters upgrading to Kubernetes `v1.35` — IPVS proxy mode is deprecated and SHOULD be migrated.

[Kubernetes kube-proxy documentation](https://kubernetes.io/docs/reference/command-line-tools-reference/kube-proxy/)

#### K8S013

**⚠️ Remediation recommended**

The Kubernetes community Ingress NGINX controller (`registry.k8s.io/ingress-nginx/controller` or `k8s.gcr.io/ingress-nginx/controller`) has been retired. Users running this controller SHOULD migrate to an actively maintained ingress controller such as the AWS Load Balancer Controller or a third-party alternative.

This check scans Deployments and DaemonSets for container images referencing the retired Ingress NGINX controller.

[Ingress NGINX Controller](https://kubernetes.github.io/ingress-nginx/)

---

## Retired Checks

The following checks have been retired and are no longer evaluated. They remain documented here for reference.

#### K8S009

!!! warning "Retired"
    This check applied to Kubernetes versions up to 1.24 and is no longer relevant for supported cluster versions.

Pod security policies were removed in Kubernetes `v1.25`. Clusters that previously relied on `PodSecurityPolicy` resources needed to migrate to the built-in Pod Security Admission controller.

[Migrate from PodSecurityPolicy to the Built-In PodSecurity Admission Controller](https://kubernetes.io/docs/tasks/configure-pod-container/migrate-from-psp/)

#### K8S010

!!! warning "Retired"
    This check applied to Kubernetes versions up to 1.24 and is no longer relevant for supported cluster versions.

The in-tree Amazon EBS storage provisioner was deprecated and clusters upgrading to version `v1.23` needed to install the Amazon EBS CSI driver first.

[Amazon EBS CSI migration frequently asked questions](https://docs.aws.amazon.com/eks/latest/userguide/ebs-csi-migration-faq.html)

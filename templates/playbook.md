# EKS Cluster Upgrade: Kubernetes `v{{ current_version }}` to `v{{ target_version }}`

|                            |                           Value                           |
| :------------------------- | :-------------------------------------------------------: |
| Current version            |                 `v{{ current_version }}`                  |
| Target version             |                  `v{{ target_version }}`                  |
| EKS Managed node group(s)  | {{#if eks_managed_node_group }} ✅ {{ else }} ➖ {{/if}}  |
| Self-Managed node group(s) | {{#if self_managed_node_group }} ✅ {{ else }} ➖ {{/if}} |
| Fargate profile(s)         |     {{#if fargate_profile }} ✅ {{ else }} ➖ {{/if}}     |
| AMI                        |    {{#if custom_ami }} Custom {{else}} Amazon {{/if}}     |

## Table of Contents

- [Preface](#preface)
    - [References](#references)
- [Pre-Upgrade](#pre-upgrade)
- [Upgrade](#upgrade)
  - [Upgrade the Control Plane](#upgrade-the-control-plane)
  - [Upgrade the Data Plane](#upgrade-the-data-plane)
{{#if eks_managed_node_group }}
    - [EKS Managed Node Group](#eks-managed-node-group)
{{/if}}
{{#if self_managed_node_group }}
    - [Self-Managed Node Group](#eks-managed-node-group)
{{/if}}
{{#if fargate_profile }}
    - [Fargate Profile](#fargate-profile)
{{/if}}
  - [Upgrade EKS Addons](#upgrade-eks-addons)
- [Post-Upgrade](#post-upgrade)

## Preface

- Unless otherwise stated, the phrase `Amazon EKS cluster` or just `cluster` throughout this document typically refers to the control plane.
- In-place cluster upgrades can only be upgraded to the next incremental minor version. For example, you can upgrade from Kubernetes version `1.20` to `1.21`, but not from `1.20` to `1.22`.
- Reverting an upgrade, or downgrading the Kubernetes version of a cluster, is not supported. If you upgrade your cluster to a new Kubernetes version and then want to revert to the previous version, you must create a new cluster and migrate your workloads.
- If the Amazon EKS cluster primary security group has been deleted, the only course of action to upgrade is to create a new cluster and migrate your workloads.
- Generally speaking, how well your cluster is configured from a high-availability perspective will determine how well your cluster handles the upgrade process. Ensuring that you have properly configured pod disruption budgets, multiple replicas specified in your deployments or statefulsets, properly configured liveness and readiness probes, etc., will help to mitigate disruptions during an upgrade. You can read more about EKS best practices for reliability [here](https://aws.github.io/aws-eks-best-practices/reliability/docs/)

### References

Prior to upgrading, review the following resources for affected changes in the next version of Kubernetes:

{{#if k8s_deprecation_url }}
- ‼️ [Kubernetes `{{ target_version }}` API deprecations]({{ k8s_deprecation_url }})
{{/if}}
- ℹ️ [Kubernetes `{{ target_version }}` release announcement]({{ k8s_release_url }})
- ℹ️ [EKS `{{ target_version }}` release notes](https://docs.aws.amazon.com/eks/latest/userguide/kubernetes-versions.html#kubernetes-{{ target_version }})

# Pre-Upgrade

1. Before updating your control plane to a new Kubernetes version, ensure that the Kubernetes minor version of data plane nodes are the same as your control plane's version.

    Control plane Kubernetes version:
    ```sh
    kubectl version --short
    ```

    Nodes Kubernetes version:
    ```sh
    kubectl get nodes
    ```

    <details>
    <summary>📌 Example</summary>

    Control plane Kubernetes version:

    ```sh
    kubectl version --short

    # Output
    Server Version: v1.21.14-eks-fb459a0
    ```

    Nodes Kubernetes version:

    ```sh
    NAME                        STATUS   ROLES    AGE     VERSION
    ip-10-0-4-19.ec2.internal   Ready    <none>   4h22m   v1.20.15-eks-abc64b1 ❌ # Needs to be upgraded first
    ip-10-0-4-23.ec2.internal   Ready    <none>   4h24m   v1.21.14-eks-fb459a0 ✅ # Ready to upgrade
    ```

    </details>

2. Verify that there are at least 5 free IPs in the VPC subnets used by the control plane. Amazon EKS creates new elastic network interfaces (ENIs) in any of the subnets specified for the control plane. If there are not enough available IPs, then the upgrade will fail (your control plane will stay on the prior version).

    ```sh
    aws ec2 describe-subnets --subnet-ids $(aws eks describe-cluster --name {{ cluster_name }} \
      --query 'cluster.resourcesVpcConfig.subnetIds' --output text) --query 'Subnets[*].AvailableIpAddressCount'
    ```

3. Ensure the security groups utilized (control plane and data plane) allow the necessary cluster communication. The new control plane network interfaces may be created in different subnets than what your existing control plane network interfaces are in, so make sure that your security group rules allow the [required cluster communication](https://docs.aws.amazon.com/eks/latest/userguide/sec-group-reqs.html) for any of the subnets that you specified when you created your cluster.

4. Check Kubernetes API versions currently in use and ensure any versions that are removed in the next Kubernetes release are updated prior to upgrading the cluster. There are several open source tools that can help you identify deprecated API versions in your Kubernetes manifests. The following open source projects support scanning both your cluster as well as manifest files to identify deprecated and/or removed API versions:

    - https://github.com/FairwindsOps/pluto
    - https://github.com/doitintl/kube-no-trouble
    - https://github.com/rikatz/kubepug

5. Ensure applications and services running on the cluster are setup for high-availability to minimize and avoid disruption during the upgrade process.
    - We strongly recommend that you have [readiness and liveness probes](https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-startup-probes/#configure-probes) configured before upgrading the data plane. This ensures that your pods register as ready/healthy at the appropriate time during an upgrade.
    - For stateless workloads
        - Specify multiple replicas for your [replica set(s)](https://kubernetes.io/docs/concepts/workloads/controllers/replicaset/)
        - Specify [pod disruption budget](https://kubernetes.io/docs/tasks/run-application/configure-pdb/) for replica sets
    - For stateful workloads
        - ℹ️ [Exploring Upgrade Strategies for Stateful Sets in Kubernetes](https://www.velotio.com/engineering-blog/exploring-upgrade-strategies-for-stateful-sets-in-kubernetes)
        - ⚠️ TODO - what guidance for cluster backup before upgrade
            - [Velero](https://github.com/vmware-tanzu/velero-plugin-for-aws)
            - [Portworx](https://github.com/portworx/aws-helm/tree/master/portworx)
        - Specify multiple replicas for your [stateful set(s)](https://kubernetes.io/docs/concepts/workloads/controllers/statefulset/)
        - Specify [pod disruption budget](https://kubernetes.io/docs/tasks/run-application/configure-pdb/) for stateful sets
            - This is useful for stateful application where there needs to be a quorum for the number of replicas to be available during an upgrade.
            - [1.24+ only - maximum unavailable pods](https://kubernetes.io/docs/concepts/workloads/controllers/statefulset/#maximum-unavailable-pods)
        - If your stateful set does not require unique ordering, typically associated with processes that utilize leader election, switching to a `parallel` strategy for [`podManagementPolicy`](https://kubernetes.io/docs/concepts/workloads/controllers/statefulset/#parallel-pod-management) will speed up your scale up/down time as well as reduce the time needed to upgrade your cluster.
        - If you are running a critical application on a Karpenter-provisioned node, such as a long running batch job or stateful application, and the node’s TTL has expired, the application will be interrupted when the instance is terminated. By adding a karpenter.sh/do-not-evict annotation to the pod, you are instructing Karpenter to preserve the node until the Pod is terminated or the do-not-evict annotation is removed. See Deprovisioning documentation for further information.
    - For batch workloads:
        - If you are running a critical application on a Karpenter-provisioned node, such as a long running batch job or stateful application, and the node’s TTL has expired, the application will be interrupted when the instance is terminated. By adding a karpenter.sh/do-not-evict annotation to the pod, you are instructing Karpenter to preserve the node until the Pod is terminated or the do-not-evict annotation is removed. See Deprovisioning documentation for further information.

6. Double check [AWS service quotas](https://docs.aws.amazon.com/general/latest/gr/aws_service_limits.html) before upgrading. Accounts that are multi-tenant or already have a number of resources provisioned may be at risk of hitting service quota limits which will cause the cluster upgrade to fail, or impede the upgrade process.

# Upgrade

The order of operations to upgrade an Amazon EKS cluster can be summarized as:

- [Upgrade the control plane](#upgrade-the-control-plane)
- [Upgrade the data plane](#upgrade-the-data-plane)
- [Upgrade EKS addons](#upgrade-eks-addons)

## Upgrade the Control Plane

- ℹ️ [Updating an Amazon EKS cluster Kubernetes version](https://docs.aws.amazon.com/eks/latest/userguide/update-cluster.html)

When upgrading the control plane, Amazon EKS performs standard infrastructure and readiness health checks for network traffic on the new control plane nodes to verify that they're working as expected. If any of these checks fail, Amazon EKS reverts the infrastructure deployment, and your cluster control plane remains on the prior Kubernetes version. Running applications aren't affected, and your cluster is never left in a non-deterministic or unrecoverable state. Amazon EKS regularly backs up all managed clusters, and mechanisms exist to recover clusters if necessary.

The control plane should be upgraded first to meet the [Kubernetes version skew policy requirements](https://kubernetes.io/releases/version-skew-policy/#kubelet) where `kubelet` must not be newer than `kube-apiserver`.

1. Upgrade the control plane to the next Kubernetes minor version:

    ```sh
    aws eks update-cluster-version --name {{ cluster_name }} --kubernetes-version {{ target_version }}
    ```

2. Wait for the control plane to finish upgrading before proceeding with any further modifications. The cluster status will change to `ACTIVE` once the upgrade is complete.

    ```sh
    aws eks describe-cluster --name {{ cluster_name }} --query 'cluster.status'
    ```

## Upgrade the Data Plane

{{#if eks_managed_node_group }}
{{ eks_managed_node_group }}
{{/if}}
{{#if self_managed_node_group }}
{{ self_managed_node_group }}
{{/if}}
{{#if fargate_profile }}
{{ fargate_profile }}
{{/if}}

## Upgrade EKS Addons

1. For each EKS addon deployed in the cluster, ensure the addon is compatible with the target Kubernetes version. If the addon is not compatible, upgrade the addon to a version that is compatible with the target Kubernetes version. You can run the following to get information on the addons used with respect to current versions:

    ```sh
    CLUSTER_NAME={{ cluster_name }}
    KUBERNETES_VERSION={{ target_version }}

    for ADDON in $(aws eks list-addons --cluster-name ${CLUSTER_NAME} --query 'addons[*]' --output text); do
        CURRENT=$(aws eks describe-addon --cluster-name ${CLUSTER_NAME} --addon-name ${ADDON} \
            --query 'addon.addonVersion' --output text)
        LATEST=$(aws eks describe-addon-versions --addon-name ${ADDON} --kubernetes-version ${KUBERNETES_VERSION} \
            --query 'addons[0].addonVersions[0].addonVersion' --output text)
        LIST=$(aws eks describe-addon-versions --addon-name ${ADDON} --kubernetes-version ${KUBERNETES_VERSION} \
            --query 'addons[0].addonVersions[:3].addonVersion')

        echo "${ADDON} current version: ${CURRENT}"
        echo "${ADDON} latest version: ${LATEST}"
        echo "${ADDON} latest 3 available versions: ${LIST}"
    done
    ```

2. Upgrade the addon to an appropriate version for the upgraded Kubernetes version:

    ```sh
    aws eks update-addon --cluster-name {{ cluster_name }} --addon-name <ADDON_NAME> --addon-version <ADDON_VERSION>
    ```

    You may need to add `--resolve-conflicts OVERWRITE` to the command if the addon has been modified since it was deployed to ensure the addon is upgraded.

# Post Upgrade

- ⚠️ Update applications running on the cluster
- ⚠️ Update tools that interact with the cluster (kubectl, awscli, etc.)

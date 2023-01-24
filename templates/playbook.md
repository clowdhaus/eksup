# EKS Cluster Upgrade

|                            |                           Value                           |
| :------------------------- | :-------------------------------------------------------: |
| Amazon EKS cluster         |                 `{{ cluster_name }}`                      |
| Current version            |                 `v{{ current_version }}`                  |
| Target version             |                  `v{{ target_version }}`                  |
| EKS Managed nodegroup(s)  | {{#if eks_managed_nodegroups }} ✅ {{ else }} ➖ {{/if}}  |
| Self-Managed nodegroup(s) | {{#if self_managed_nodegroup }} ✅ {{ else }} ➖ {{/if}} |
| Fargate profile(s)         |     {{#if fargate_profile }} ✅ {{ else }} ➖ {{/if}}     |

## Table of Contents

- [Upgrade the Control Plane](#upgrade-the-control-plane)
    - [Pre-Upgrade](#pre-upgrade)
    - [Upgrade](#upgrade)
- [Upgrade the Data Plane](#upgrade-the-data-plane)
{{#if eks_managed_nodegroups }}
    - [Pre-Upgrade](#pre-upgrade-1)
    - [Upgrade](#upgrade-1)
        - [EKS Managed Node Group](#eks-managed-node-group)
{{/if}}
{{#if self_managed_nodegroup }}
        - [Self-Managed Node Group](#eks-managed-node-group)
{{/if}}
{{#if fargate_profile }}
        - [Fargate Profile](#fargate-profile)
{{/if}}
- [Upgrade EKS Addons](#upgrade-eks-addons)
    - [Pre-Upgrade](#addon-pre-upgrade)
    - [Upgrade](#addon-upgrade)
- [Post-Upgrade](#post-upgrade)
- [References](#references)


## Upgrade the Control Plane

### Pre-Upgrade

1. Review the following resources for affected changes in the next version of Kubernetes:

{{#if k8s_deprecation_url }}
    - ‼️ [Kubernetes `{{ target_version }}` API deprecations]({{ k8s_deprecation_url }})
{{/if}}
    - ℹ️ [Kubernetes `{{ target_version }}` release announcement]({{ k8s_release_url }})
    - ℹ️ [EKS `{{ target_version }}` release notes](https://docs.aws.amazon.com/eks/latest/userguide/kubernetes-versions.html#kubernetes-{{ target_version }})

2. Per the [Kubernetes version skew policy states](https://kubernetes.io/releases/version-skew-policy/#supported-version-skew) that `kubelet` version must not be newer than `kube-apiserver`, and may be up to two minor versions older. It is recommended that the nodes in the data plane are aligned with the same minor version as the control plane; at minimum, they must not be 2 versions older than the control plane (updating the control plane first would put the version skew at 3 versions older which violates the skew policy).

    <details>
    <summary>📌 CLI Example</summary>

    Ensure you have updated your `kubeconfig` locally before executing the following commands:

    ```sh
    aws eks update-kubeconfig --region {{ region }}  --name {{ cluster_name }}
    ```

    Control plane Kubernetes version:

    ```sh
    kubectl version --short

    # Output
    Server Version: v1.21.14-eks-fb459a0
    ```

    Node(s) Kubernetes version(s):

    ```sh
    kubectl get nodes

    # Output
    NAME                        STATUS   ROLES    AGE     VERSION
    ip-10-0-4-19.ec2.internal   Ready    <none>   4h22m   v1.20.15-eks-abc64b1 ❌ # Needs to be upgraded first
    ip-10-0-4-23.ec2.internal   Ready    <none>   4h24m   v1.21.14-eks-fb459a0 ✅ # Ready to upgrade
    ```
    </details>

    ##### 📝 Analysis Results
{{ version_skew }}

3. Verify that there are at least 5 free IPs in the VPC subnets used by the control plane. Amazon EKS creates new elastic network interfaces (ENIs) in any of the subnets specified for the control plane. If there are not enough available IPs, then the upgrade will fail (your control plane will stay on the prior version).

    <details>
    <summary>📌 CLI Example</summary>

    ```sh
    aws ec2 describe-subnets --region {{ region }} --subnet-ids \
        $(aws eks describe-cluster --region {{ region }} --name {{ cluster_name }} \
      --query 'cluster.resourcesVpcConfig.subnetIds' --output text) \
      --query 'Subnets[*].AvailableIpAddressCount'
    ```

    </details>

    ##### 📝 Analysis Results
{{ control_plane_ips }}

4. Ensure the cluster is free of any health issues as reported by Amazon EKS. If there are any issues, resolution of those issues is required before upgrading the cluster. Note - resolution in some cases may require creating a new cluster. For example, if the cluster primary security group was deleted, at this time, the only course of remediation is to create a new cluster and migrate any workloads over to that cluster (treated as a blue/green cluster upgrade).

    <details>
    <summary>📌 CLI Example</summary>

    ```sh
    aws eks describe-cluster --region {{ region }} --name {{ cluster_name }} \
        --query 'cluster.health'
    ```

    </details>

    ##### 📝 Analysis Results
{{ cluster_health }}

5. Ensure the EKS addons in use are using a version that is supported by the intended target Kubernetes version. If an addon is not compatible with the intended target Kubernetes version, upgrade the addon to a version that is compatible before upgrading the cluster.

    <details>
    <summary>📌 CLI Example</summary>

    ```sh
    for ADDON in $(aws eks list-addons --cluster-name {{ cluster_name }} \
        --region {{ region }} --query 'addons[*]' --output text); do
      CURRENT=$(aws eks describe-addon --cluster-name {{ cluster_name }} --region {{ region }} \
        --addon-name ${ADDON} --query 'addon.addonVersion' --output text)
      LATEST=$(aws eks describe-addon-versions --region {{ region }} --addon-name ${ADDON} \
        --kubernetes-version {{ target_version }} --query 'addons[0].addonVersions[0].addonVersion' --output text)
      LIST=$(aws eks describe-addon-versions --region {{ region }} --addon-name ${ADDON} \
        --kubernetes-version {{ target_version }} --query 'addons[0].addonVersions[:3].addonVersion')

      echo "${ADDON} current version: ${CURRENT}"
      echo "${ADDON} latest version: ${LATEST}"
      echo "${ADDON} latest 3 available versions: ${LIST}"
    done
    ```

    </details>

    ##### 📝 Analysis Results
{{ addon_version_compatibility }}

5. Check Kubernetes API versions currently in use and ensure any versions that are removed in the next Kubernetes release are updated prior to upgrading the cluster. There are several open source tools that can help you identify deprecated API versions in your Kubernetes manifests. The following open source projects support scanning both your cluster as well as manifest files to identify deprecated and/or removed API versions:

    - https://github.com/FairwindsOps/pluto
    - https://github.com/doitintl/kube-no-trouble

### Upgrade

ℹ️ [Updating an Amazon EKS cluster Kubernetes version](https://docs.aws.amazon.com/eks/latest/userguide/update-cluster.html)

When upgrading the control plane, Amazon EKS performs standard infrastructure and readiness health checks for network traffic on the new control plane nodes to verify that they're working as expected. If any of these checks fail, Amazon EKS reverts the infrastructure deployment, and your cluster control plane remains on the prior Kubernetes version. Running applications aren't affected, and your cluster is never left in a non-deterministic or unrecoverable state. Amazon EKS regularly backs up all managed clusters, and mechanisms exist to recover clusters if necessary.

1. Upgrade the control plane to the next Kubernetes minor version:

    ```sh
    aws eks update-cluster-version --region {{ region }} --name {{ cluster_name }} \
        --kubernetes-version {{ target_version }}
    ```

2. Wait for the control plane to finish upgrading before proceeding with any further modifications. The cluster status will change to `ACTIVE` once the upgrade is complete.

    ```sh
    aws eks describe-cluster --region {{ region }} --name {{ cluster_name }} \
        --query 'cluster.status'
    ```

## Upgrade the Data Plane

### Pre-Upgrade

1. Ensure applications and services running on the cluster are setup for high-availability to minimize and avoid disruption during the upgrade process.
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

2. Double check [AWS service quotas](https://docs.aws.amazon.com/general/latest/gr/aws_service_limits.html) before upgrading. Accounts that are multi-tenant or already have a number of resources provisioned may be at risk of hitting service quota limits which will cause the cluster upgrade to fail, or impede the upgrade process.

{{#if eks_managed_nodegroups }}
{{ eks_managed_nodegroup_template }}
{{/if}}
{{#if self_managed_nodegroup }}
{{ self_managed_nodegroup }}
{{/if}}
{{#if fargate_profile }}
{{ fargate_profile }}
{{/if}}

## Upgrade EKS Addons

### <a name="addon-pre-upgrade"></a>Pre-Upgrade

1. Ensure the EKS addons in use are free of any health issues as reported by Amazon EKS. If there are any issues, resolution of those issues is required before upgrading the cluster.

    <details>
    <summary>📌 CLI Example</summary>

    ```sh
    aws eks describe-addon --region {{ region }} --cluster-name {{ cluster_name }} \
        --addon-name <ADDON_NAME> --query 'addon.health'
    ```

    </details>

    ##### 📝 Analysis Results
{{ addon_health }}

### <a name="addon-pre-upgrade"></a>Upgrade

1. Upgrade the addon to an appropriate version for the upgraded Kubernetes version:

    ```sh
    aws eks update-addon --region {{ region }} --cluster-name {{ cluster_name }} \
        --addon-name <ADDON_NAME> --addon-version <ADDON_VERSION>
    ```

    You may need to add `--resolve-conflicts OVERWRITE` to the command if the addon has been modified since it was deployed to ensure the addon is upgraded.

## Post Upgrade

- ⚠️ Update applications running on the cluster
- ⚠️ Update tools that interact with the cluster (kubectl, awscli, etc.)

## References

### Symbol Table

| Symbol | Description |
| :----: | :---------- |
| 📝     | The results generated by analyzing the cluster |
| ℹ️     | Informational - users are encouraged to familiarize themselves with the information but no action is required to upgrade  |
| ⚠️     | Recommended - users are encouraged to evaluate the recommendation and determine if it is applicable and whether or not to act upon that recommendation. A recommendation is not required to be remediated prior to upgrading |
| ‼️     | Required - users must remediate the requirements specified prior to upgrading to avoid downtime and/or disruption during the upgrade process |

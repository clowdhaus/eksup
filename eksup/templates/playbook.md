# EKS Cluster Upgrade

|                            |                           Value                           |
| :------------------------- | :-------------------------------------------------------: |
| Amazon EKS cluster         |                 `{{ cluster_name }}`                      |
| Current version            |                 `v{{ current_version }}`                  |
| Target version             |                  `v{{ target_version }}`                  |
| EKS Managed nodegroup(s)  | {{#if data_plane_findings.eks_managed_nodegroups }} ✅ {{ else }} ➖ {{/if}}  |
| Self-Managed nodegroup(s) | {{#if data_plane_findings.self_managed_nodegroups }} ✅ {{ else }} ➖ {{/if}} |
| Fargate profile(s)         |     {{#if data_plane_findings.fargate_profiles }} ✅ {{ else }} ➖ {{/if}}     |

## Table of Contents

- [Upgrade the Control Plane](#upgrade-the-control-plane)
    - [Control Plane Pre-Upgrade](#control-plane-pre-upgrade)
    - [Control Plane Upgrade](#control-plane-upgrade)
- [Upgrade the Data Plane](#upgrade-the-data-plane)
{{#if data_plane_findings.eks_managed_nodegroups }}
    - [Data Plane Pre-Upgrade](#data-plane-pre-upgrade)
        - [EKS Managed Nodegroup](#eks-managed-nodegroup)
{{/if}}
{{#if data_plane_findings.self_managed_nodegroups }}
        - [Self-Managed Nodegroup](#self-managed-nodegroup)
{{/if}}
{{#if data_plane_findings.fargate_profiles }}
        - [Fargate Profile](#fargate-profile)
{{/if}}
- [Upgrade EKS Addons](#upgrade-eks-addons)
    - [Addon Pre-Upgrade](#addon-pre-upgrade)
    - [Addon Upgrade](#addon-upgrade)
- [Post-Upgrade](#post-upgrade)
- [References](#references)


## Upgrade the Control Plane

### Control Plane Pre-Upgrade

1. Review the following resources for affected changes in the next version of Kubernetes:

{{#if k8s_deprecation_url }}
    - ‼️ [Kubernetes `{{ target_version }}` API deprecations]({{ k8s_deprecation_url }})
{{/if}}
    - ℹ️ [Kubernetes `{{ target_version }}` release announcement]({{ k8s_release_url }})
    - ℹ️ [EKS `{{ target_version }}` release notes](https://docs.aws.amazon.com/eks/latest/userguide/kubernetes-versions.html#kubernetes-{{ target_version }})

2. Per the [Kubernetes version skew policy](https://kubernetes.io/releases/version-skew-policy/#supported-version-skew), the `kubelet` version must not be newer than `kube-apiserver`, and may be up to two minor versions older. It is recommended that the nodes in the data plane are aligned with the same minor version as the control plane before upgrading.

    <details>
    <summary>📌 CLI Example</summary>

    Ensure you have updated your `kubeconfig` locally before executing the following commands:

    ```sh
    aws eks update-kubeconfig --region {{ region }}  --name {{ cluster_name }}
    ```

    Control plane Kubernetes version:

    ```sh
    kubectl version --short

    # Output (truncated)
    Server Version: v1.23.14-eks-ffeb93d
    ```

    Node(s) Kubernetes version(s):

    ```sh
    kubectl get nodes

    # Output
    NAME                                  STATUS   ROLES    AGE   VERSION
    fargate-ip-10-0-14-253.ec2.internal   Ready    <none>   9h    v1.23.14-eks-a1bebd3 ✅ # Ready to upgrade
    fargate-ip-10-0-7-182.ec2.internal    Ready    <none>   9h    v1.23.14-eks-a1bebd3 ✅ # Ready to upgrade
    ip-10-0-14-102.ec2.internal           Ready    <none>   9h    v1.22.15-eks-fb459a0 ⚠️ # Recommended to upgrade first
    ip-10-0-27-61.ec2.internal            Ready    <none>   9h    v1.22.15-eks-fb459a0 ⚠️ # Recommended to upgrade first
    ip-10-0-41-36.ec2.internal            Ready    <none>   9h    v1.21.14-eks-fb459a0 ❌ # Requires upgrade first
    ```
    </details>

    #### Check [[K8S001]](https://clowdhaus.github.io/eksup/process/checks/#k8s001)
{{ data_plane_findings.version_skew }}

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

    #### Check [[EKS001]](https://clowdhaus.github.io/eksup/process/checks/#eks001)
{{ control_plane_ips }}

4. Ensure the cluster is free of any health issues as reported by Amazon EKS. If there are any issues, resolution of those issues is required before upgrading the cluster. Note - resolution in some cases may require creating a new cluster. For example, if the cluster primary security group was deleted, at this time, the only course of remediation is to create a new cluster and migrate any workloads over to that cluster (treated as a blue/green cluster upgrade).

    <details>
    <summary>📌 CLI Example</summary>

    ```sh
    aws eks describe-cluster --region {{ region }} --name {{ cluster_name }} \
        --query 'cluster.health'
    ```

    </details>

    #### Check [[EKS002]](https://clowdhaus.github.io/eksup/process/checks/#eks002)
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
        --kubernetes-version {{ target_version }} --query 'addons[0].addonVersions[*].addonVersion')

      echo "${ADDON} current version: ${CURRENT}"
      echo "${ADDON} next latest version: ${LATEST}"
      echo "${ADDON} next available versions: ${LIST}"
    done
    ```

    </details>

    #### Check [[EKS005]](https://clowdhaus.github.io/eksup/process/checks/#eks005)
{{ addon_version_compatibility }}

5. Check Kubernetes API versions currently in use and ensure any versions that are removed in the next Kubernetes release are updated prior to upgrading the cluster. There are several open source tools that can help you identify deprecated API versions in your Kubernetes manifests. The following open source projects support scanning both your cluster as well as manifest files to identify deprecated and/or removed API versions:

    - https://github.com/FairwindsOps/pluto
    - https://github.com/doitintl/kube-no-trouble

### Control Plane Upgrade

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

### Data Plane Pre-Upgrade

1. Ensure applications and services running on the cluster are setup for high-availability to minimize and avoid disruption during the upgrade process.

    🚧 TODO - fill in analysis results

    #### Check [[K8S002]](https://clowdhaus.github.io/eksup/process/checks/#k8s002)
{{ kubernetes_findings.min_replicas }}

    #### Check [[K8S003]](https://clowdhaus.github.io/eksup/process/checks/#k8s003)
{{ kubernetes_findings.min_ready_seconds }}

2. Inspect [AWS service quotas](https://docs.aws.amazon.com/general/latest/gr/aws_service_limits.html) before upgrading. Accounts that are multi-tenant or already have a number of resources provisioned may be at risk of hitting service quota limits which will cause the cluster upgrade to fail, or impede the upgrade process.

{{#if pod_ips}}
3. Verify that there is sufficient IP space available to the pods running in the cluster when using custom networking. With the in-place, surge upgrade process, there will be higher IP consumption during the upgrade.

    <details>
    <summary>📌 CLI Example</summary>

    Ensure you have updated your `kubeconfig` locally before executing the following commands:

    ```sh
    aws eks update-kubeconfig --region {{ region }}  --name {{ cluster_name }}
    ```

    Get the number of available IPs in each subnet used by the custom networking `ENIConfig` resources:
    ```sh
    aws ec2 describe-subnets --region {{ region }} --subnet-ids \
        $(kubectl get ENIConfigs -n kube-system -o jsonpath='{.items[*].spec.subnet}') \
        --query 'Subnets[*].AvailableIpAddressCount'
    ```

    </details>

    #### Check [[AWS002]](https://clowdhaus.github.io/eksup/process/checks/#aws002)
{{ pod_ips }}
{{/if}}

{{#if data_plane_findings.eks_managed_nodegroups }}
{{ eks_managed_nodegroup_template }}
{{/if}}
{{#if data_plane_findings.self_managed_nodegroups }}
{{ self_managed_nodegroup_template }}
{{/if}}
{{#if data_plane_findings.fargate_profiles }}
{{ fargate_profile_template }}
{{/if}}

## Upgrade EKS Addons

### Addon Pre-Upgrade

1. Ensure the EKS addons in use are free of any health issues as reported by Amazon EKS. If there are any issues, resolution of those issues is required before upgrading the cluster.

    <details>
    <summary>📌 CLI Example</summary>

    ```sh
    aws eks describe-addon --region {{ region }} --cluster-name {{ cluster_name }} \
        --addon-name <ADDON_NAME> --query 'addon.health'
    ```

    </details>

    #### Check [[EKS004]](https://clowdhaus.github.io/eksup/process/checks/#eks004)
{{ addon_health }}

### Addon Upgrade

1. Upgrade the addon to an appropriate version for the upgraded Kubernetes version:

    ```sh
    aws eks update-addon --region {{ region }} --cluster-name {{ cluster_name }} \
        --addon-name <ADDON_NAME> --addon-version <ADDON_VERSION>
    ```

    You may need to add `--resolve-conflicts OVERWRITE` to the command if the addon has been modified since it was deployed to ensure the addon is upgraded.

## Post Upgrade

- Update applications running on the cluster
- Update tools that interact with the cluster (kubectl, awscli, etc.)

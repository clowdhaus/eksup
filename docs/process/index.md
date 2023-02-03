# Getting Started

## Preface

- Unless otherwise stated, the phrase `Amazon EKS cluster` or just `cluster` throughout the documentation typically refers to the control plane.
- In-place cluster upgrades can only be upgraded to the next incremental minor version. For example, you can upgrade from Kubernetes version `1.20` to `1.21`, but not from `1.20` to `1.22`.
- Reverting an upgrade, or downgrading the Kubernetes version of a cluster, is not supported. If you upgrade your cluster to a newer Kubernetes version and then want to revert to the previous version, you must create a new,separate cluster and migrate your workloads.
- If the Amazon EKS cluster primary security group has been deleted, the only course of action to upgrade is to create a new, separate cluster and migrate your workloads.
- Generally speaking, how well your cluster is configured from a high-availability perspective will determine how well your cluster handles the upgrade process. Ensuring that you have properly configured pod disruption budgets, multiple replicas specified in your deployments and statefulsets, properly configured readiness probes, etc., will help to mitigate potential disruptions or downtime during an upgrade. You can read more about EKS best practices for reliability [here](https://aws.github.io/aws-eks-best-practices/reliability/docs/).

## Terminology

- **In-place upgrade**: the process of upgrading the associated resource without re-creation. An in-place cluster upgrade means the Amazon EKS cluster is updated while it continues running and serving traffic and it will retain all of its unique attributes such as endpoint, OIDC thumbprints, etc. An in-place nodegroup upgrade consists of upgrading the underlying EC2 instances within the nodegroup without modifying the nodegroup itself. Typically this is performed by providing a new launch template version that contains the new/updated AMI to the nodegroup which will trigger the nodegroup to replace the EC2 instances with new instances to match the updated launch template. In-place upgrades generally use fewer resources than blue/green upgrades, and are typically easier to perform. A downside to an in-place upgrade is the inability to rollback to the prior configuration in some instances, or a longer time to complete the rollback to a prior configuration.
- **Blue/green upgrade**: an upgrade strategy where a second resource (green) is created alongside the current resource (blue). Once the new, second (green) resource is ready, workloads and traffic are shifted from the current resource (blue) over to the new, second (green) resource. Once the workloads and traffic have been shifted over, the current resource (blue) is deleted. Blue/green upgrades allow for better risk mitigation during upgrades, especially when there are substantial changes made in the upgrade since the new, second (green) resource can be tested and validate out of band without disrupting workloads or traffic. Only once the new, second (green) resource has been validated should the workload and traffic be shifted over. In addition, if any unforeseen issues do arise once workloads and traffic are on the new, second (green) resource, rollbacks are generally quick and easy to perform since the prior (blue) resource is still available. A downside to a blue/green upgrade is the additional resources required to create the second resource (green) and the additional time required to perform the upgrade. It can take quite a bit more effort to architect and orchestrate blue/green upgrades but the benefits can be well worth it. You can utilize the blue/green upgrade strategy for the entire cluster as well as for nodegroups. See below for an overview on the process of performing a blue/green upgrade on a nodegroup.

## Overview

A high-level overview of the process for upgrading an Amazon EKS cluster in-place consists of:

1. Check for things that will affect the control plane upgrade.
    - This includes checking for any deprecated or removed Kubernetes API objects, ensuring there are at least 5 available IP addresses in the subnets used by the control plane, ensuring the version skew between the control plane and the data plane is within the supported range, etc.
2. Upgrade the control plane to the next incremental minor version of Kubernetes
    - This process will take approximately 15 minutes to complete. Even though Amazon EKS runs a highly available control plane, you might experience minor service interruptions during an update. For example, assume that you attempt to connect to an API server around when it's terminated and replaced by a new API server that's running the new version of Kubernetes. You might experience API call errors or connectivity issues. If this happens, retry your API operations until they succeed.
3. Check for things that will affect the data plane upgrade.
    - This includes checking for any reported health issues on the nodegroups, ensuring there are enough available IPs to perform the upgrade, ensuring the applications running on the data plane are configured for high availability using pod disruption budgets, readiness probes, topology spread constraints, etc.
4. Update the data plane to match the new control plane Kubernetes version
    - Update nodegroups to roll out new AMIs that match the new control plane Kubernetes version; cordon and drain Fargate nodes to have them replaced with new nodes that match the new control plane Kubernetes version
5. Check for any reported health issues on the EKS addons
6. Update the EKS addons
    - Ensure the addons are using a version within the supported range for the new control plane Kubernetes version; ideally, use the default version for the new control plane Kubernetes version
7. Update applications running on the cluster
    - Some application such as `cluster-autoscaler`, have a versioning scheme that aligns with the Kubernetes version of the cluster they are running on. This means that once the control plane and data plane components have been updated, these applications should be updated to match as well
8. Update any clients that interact with the cluster
    - This includes updating the `kubectl` client to match the new control plane Kubernetes version

This is just a brief overview of the general process for upgrading an Amazon EKS cluster in-place. There are a number of finer details that need to be checked and considered on a per-upgrade basis to ensure the upgrade is successful. This is why `eksup` was created - to help surface that information to users and provide guidance on the process to upgrade their cluster.

### Symbol Table

| Symbol | Description |
| :----: | :---------- |
| ℹ️     | Informational - users are encouraged to familiarize themselves with the information but no action is required to upgrade  |
| ⚠️     | Recommended - users are encouraged to evaluate the recommendation and determine if it is applicable and whether or not to act upon that recommendation. Not remediating the finding does not prevent the upgrade from occurring. |
| ❌     | Required - users must remediate the finding prior to upgrading to be able to perform the upgrade and avoid downtime or disruption |

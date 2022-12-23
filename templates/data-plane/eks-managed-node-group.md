### EKS Managed Node Group

- ℹ️ [Updating a managed node group](https://docs.aws.amazon.com/eks/latest/userguide/update-managed-node-group.html)
- ℹ️ [Managed node group update behavior](https://docs.aws.amazon.com/eks/latest/userguide/managed-node-update-behavior.html)

- It is recommended to configure the [node group update config](https://docs.aws.amazon.com/eks/latest/APIReference/API_NodegroupUpdateConfig.html) to support updating multiple nodes in parallel during an upgrade. The update config has a max quota of 100 nodes that can be updated in parallel at once. It is recommended to start with an update configuration of 30% max unavailable percentage and adjust as necessary. Increasing this percentage will reduce the time to upgrade (until the max quota of 100 nodes is reached) but also increase the amount of churn within then node group and therefore increasing the potential for disruption to services running on the nodes. Conversely, reducing the percentage will increase the time to upgrade but also reduce the amount of churn within the node group and therefore reduce the potential for disruption to services running on the nodes. Users should test the impact of the update configuration on their workloads and adjust as necessary to balance between time to upgrade and potential risk for service disruption.

- The default update strategy is a rolling update. This option respects the pod disruption budgets for your cluster. Updates fail if there's a pod disruption budget issue that prevents Amazon EKS from gracefully draining the pods that are running on the node group, or if pods do not safely evict from the nodes within a 15 minute window after the node has been marked as cordoned and set to drain. To circumvent this, you can specify a force update. This option does not respect pod disruption budgets. Updates occur regardless of pod disruption budget issues by forcing node restarts to occur.

#### Upgrade

{{#if is_custom_ami }}
1. Update the launch template, specifying the ID of an AMI that matches the control plane's Kubernetes version:

    ```sh
    aws ec2 create-launch-template-version --launch-template-id <LAUNCH_TEMPLATE_ID> \
      --source-version <LAUNCH_TEMPLATE_VERSION> --launch-template-data 'ImageId=<AMI_ID>'
    ```

2. Update the launch template version specified on the EKS managed node group:

    ```sh
    aws eks update-nodegroup-version --cluster-name <CLUSTER_NAME> \
      --nodegroup-name <NODEGROUP_NAME> --launch-template <LAUNCH_TEMPLATE>
    ```

{{else}}
1. Update the Kubernetes version specified on the EKS managed node group:

    ```sh
    aws eks update-nodegroup-version --cluster-name <CLUSTER_NAME> \
      --nodegroup-name <NODEGROUP_NAME> --kubernetes-version {{ target_version }}
    ```
{{/if}}

When a node is upgraded, the following happens with the Pods:

  - The node is cordoned so that Kubernetes does not schedule new Pods on it.
  - The node is then drained while respecting the set `PodDisruptionBudget` and `GracefulTerminationPeriod` settings for pods for up to 15 minutes.
  - The control plane reschedules Pods managed by controllers onto other nodes. Pods that cannot be rescheduled stay in the Pending phase until they can be rescheduled.

The node pool upgrade process may take up to a few hours depending on the upgrade strategy, the number of nodes, and their workload configurations. Configurations that can cause a node upgrade to take longer to complete include:

  - A high value of `terminationGracePeriodSeconds` in a Pod's configuration.
  - A conservative Pod Disruption Budget.
  - Node affinity interactions
  - Attached PersistentVolumes

In the event that you encounter pod disruption budget issues or update timeouts due to pods not safely evicting from the nodes within the 15 minute window, you can force the update to proceed by adding the `--force` flag.

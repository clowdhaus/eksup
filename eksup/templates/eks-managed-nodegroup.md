#### EKS Managed Nodegroup

ℹ️ [Updating a managed nodegroup](https://docs.aws.amazon.com/eks/latest/userguide/update-managed-node-group.html)

ℹ️ [Managed nodegroup update behavior](https://docs.aws.amazon.com/eks/latest/userguide/managed-node-update-behavior.html)

The [nodegroup update config](https://docs.aws.amazon.com/eks/latest/APIReference/API_NodegroupUpdateConfig.html) supports updating multiple nodes, up to a max of 100 nodes, in parallel during an upgrade. It is recommended to start with an update configuration of 30% max unavailable percentage and adjust as necessary. Increasing this percentage will reduce the time to upgrade (until the max quota of 100 nodes is reached) but also increase the amount of churn within then nodegroup and therefore increasing the potential for disruption to services running on the nodes. Conversely, reducing the percentage will increase the time to upgrade but also reduce the amount of churn within the nodegroup and therefore reduce the potential for disruption to services running on the nodes. Users should test the impact of the update configuration on their workloads and adjust as necessary to balance between time to upgrade and potential risk for service disruption.

The default update strategy for EKS managed nodegroups is a surge, rolling update which respects the pod disruption budgets for your cluster. Updates can fail if there's a pod disruption budget issue that prevents Amazon EKS from gracefully draining the pods that are running on the nodegroup, or if pods do not safely evict from the nodes within a 15 minute window after the node has been marked as cordoned and set to drain. To circumvent this, you can specify a force update which does *NOT* respect pod disruption budgets. Updates occur regardless of pod disruption budget issues by forcing node replacements.

##### Pre-Upgrade

1. Ensure the EKS managed nodegroup(s) are free of any health issues as reported by Amazon EKS. If there are any issues, resolution of those issues is required before upgrading the cluster.

    <details>
    <summary>📌 CLI Example</summary>

    ```sh
    aws eks describe-nodegroup --region {{ region }} --cluster-name {{ cluster_name }} \
      --nodegroup-name <NAME> --query 'nodegroup.health'
    ```

    </details>

    #### Check [[EKS003]](https://clowdhaus.github.io/eksup/info/checks/#eks003)
{{ eks_managed_nodegroup_health }}

2. Ensure the EKS managed nodegroup(s) do not have any pending updates and they are using the latest version of their respective launch templates. If the nodegroup(s) are not using the latest launch template, it is recommended to update to the latest to avoid accidentally introducing any additional and un-intended changes during the upgrade.

    <details>
    <summary>📌 CLI Example</summary>

    ```sh
    // TODO
    ```

    </details>

    Check [[EKS006]](https://clowdhaus.github.io/eksup/info/checks/#eks006)
{{ eks_managed_nodegroup_update }}

##### Upgrade

The following steps are applicable for each nodegroup in the cluster.

Custom AMI:

  1. Update the launch template, specifying the ID of an AMI that matches the control plane's Kubernetes version:

      ```sh
      aws ec2 create-launch-template-version --region {{ region }} \
        --launch-template-id <LAUNCH_TEMPLATE_ID> \
        --source-version <LAUNCH_TEMPLATE_VERSION> --launch-template-data 'ImageId=<AMI_ID>'
      ```

  2. Update the launch template version specified on the EKS managed nodegroup:

      ```sh
      aws eks update-nodegroup-version --region {{ region }} --cluster-name {{ cluster_name }} \
        --nodegroup-name <NODEGROUP_NAME> --launch-template <LAUNCH_TEMPLATE>
      ```


EKS optimized AMI provided by Amazon EKS:

  1. Update the Kubernetes version specified on the EKS managed nodegroup:

      ```sh
      aws eks update-nodegroup-version --region {{ region }} --cluster-name {{ cluster_name }} \
        --nodegroup-name <NODEGROUP_NAME> --kubernetes-version {{ target_version }}
      ```

##### Process

The following events take place when a nodegroup detects changes that require nodes to be cycled and replaced, such as upgrading the Kubernetes version or deploying a new AMI:

For each node in the nodegroup:
  - The node is cordoned so that Kubernetes does not schedule new Pods on it.
  - The node is then drained while respecting the set `PodDisruptionBudget` and `GracefulTerminationPeriod` settings for pods for up to 15 minutes.
  - The control plane reschedules Pods managed by controllers onto other nodes. Pods that cannot be rescheduled stay in the Pending phase until they can be rescheduled.

The node pool upgrade process may take up to a few hours depending on the upgrade strategy, the number of nodes, and their workload configurations. Configurations that can cause a node upgrade to take longer to complete include:

  - A high value of `terminationGracePeriodSeconds` in a Pod's configuration.
  - A conservative Pod Disruption Budget.
  - Node affinity interactions
  - Attached PersistentVolumes

In the event that you encounter pod disruption budget issues or update timeouts due to pods not safely evicting from the nodes within the 15 minute window, you can force the update to proceed by adding the `--force` flag.

#### [EKS Managed Node Group](https://docs.aws.amazon.com/eks/latest/userguide/update-managed-node-group.html)

##### Before Upgrading

- Refer to the official EKS documentation for [updating EKS managed node groups](https://docs.aws.amazon.com/eks/latest/userguide/update-managed-node-group.html) as well as the documentation on the EKS managed node group [update behavior](https://docs.aws.amazon.com/eks/latest/userguide/managed-node-update-behavior.html).

- It is recommended to configure the [node group update config](https://docs.aws.amazon.com/eks/latest/APIReference/API_NodegroupUpdateConfig.html) to support updating multiple nodes in parallel during an upgrade. The update config has a max quota of 100 nodes that can be updated in parallel at once. A recommended starting point for the configuration is to use a value of 30% as the max unavailable percentage and adjust as necessary.

- The default update strategy is a rolling update. This option respects the pod disruption budgets for your cluster. Updates fail if there's a pod disruption budget issue that causes Amazon EKS to be unable to gracefully drain the pods that are running on this node group, or if pods do not safely evict from the nodes within a 15 minute window after the node has been marked as cordoned and set to drain. You can specify a force update. This option does not respect pod disruption budgets. Updates occur regardless of pod disruption budget issues by forcing node restarts to occur.

##### Upgrade

To upgrade an EKS managed node group:

{{#if is_custom_ami }}
1. Update the launch template, specifying the ID of an AMI that matches the control plane's Kubernetes version:

```sh
aws ec2 create-launch-template-version --launch-template-id <ID> \
  --source-version <LT_VERSION> --launch-template-data 'ImageId=<AMI_ID>'
```

2. Update the launch template version specified on the EKS managed node group:

```sh
aws eks update-nodegroup-version --region <REGION> --cluster-name <CLUSTER_NAME> --nodegroup-name <NODEGROUP_NAME> --launch-template <LAUNCH_TEMPLATE>
```

{{else}}
1. Update the Kubernetes version specified on the EKS managed node group:

```sh
aws eks update-nodegroup-version --region <REGION> --cluster-name <CLUSTER_NAME> --nodegroup-name <NODEGROUP_NAME> --kubernetes-version {{ target_version }}
```
{{/if}}

In the event that you encounter pod disruption budget issues or update timeouts due to pods not safely evicting from the nodes within the 15 minute window, you can force the update to proceed by adding the `--force` flag.


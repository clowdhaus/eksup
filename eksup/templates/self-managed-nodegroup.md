#### Self-Managed Nodegroup

ℹ️ [Self-managed node updates](https://docs.aws.amazon.com/eks/latest/userguide/update-workers.html)

[Instance refresh](https://docs.aws.amazon.com/autoscaling/ec2/userguide/asg-instance-refresh.html) functionality provided by AWS Auto Scaling groups should be utilized on self-managed nodegroups in coordination with the [`node-termination-handler`](https://github.com/aws/aws-node-termination-handler) to gracefully migrate pods from instances scheduled for replacement when upgrading. Once the launch template has been updated with the new AMI ID, the Auto Scaling group will initiate the instance refresh cycle to rollout the replacement of instances to meet the new launch template specification. The `node-termination-handler` listens to the Auto Scaling group lifecycle events to intervene and gracefully migrate pods off of the instance(s) being replaced. When using EKS managed node groups, this functionality (rolling nodes with instance refresh and gracefully migrating pods with `node-termination-handler`) are provided by the service.

A starting point for the instance refresh configuration is to use a value of 70% as the minimum healthy percentage and adjust as necessary. Lowering this value will allow more instances to be refreshed at once, however, it will also increase the risk of overwhelming the control plane with requests. Users should aim to replace no more than 100 instances at a time to match the behavior of EKS managed node groups and avoid overwhelming the control plane during an upgrade.

##### Pre-Upgrade

1. Ensure the self-managed nodegroup(s) do not have any pending updates and they are using the latest version of their respective launch templates. If the nodegroup(s) are not using the latest launch template, it is recommended to update to the latest to avoid accidentally introducing any additional and un-intended changes during the upgrade.

    <details>
    <summary>📌 CLI Example</summary>

    ```sh
    // TODO
    ```

    </details>

    ##### 📝 [EKS007] Analysis Results
{{ self_managed_nodegroup_update }}

##### Upgrade

1. Update the launch template, specifying the ID of an AMI that matches the control plane's Kubernetes version:

    ```sh
    aws ec2 create-launch-template-version --region {{ region }}  \
      --launch-template-id <LAUNCH_TEMPLATE_ID> \
      --source-version <LAUNCH_TEMPLATE_VERSION> --launch-template-data 'ImageId=<AMI_ID>'
    ```

    You can [retrieve the recommended EKS optimized AL2 AMI ID](https://docs.aws.amazon.com/eks/latest/userguide/retrieve-ami-id.html) by running the following command:

    ```sh
    aws ssm get-parameter --region {{ region }} \
      --name /aws/service/eks/optimized-ami/{{ target_version }}/amazon-linux-2/recommended/image_id \
      --query 'Parameter.Value' --output text
    ```

2. Update the autoscaling-group to use the new launch template

    ```sh
    aws autoscaling update-auto-scaling-group --region {{ region }}
      --auto-scaling-group-name <ASG_NAME> \
      --launch-template LaunchTemplateId=<LAUNCH_TEMPLATE_ID>,Version='$Latest'
    ```

3. Wait for the instance refresh to complete. From the [documentation](https://docs.aws.amazon.com/autoscaling/ec2/userguide/asg-instance-refresh.html#instance-refresh-how-it-works), here is what happens during the instance refresh:

    > Amazon EC2 Auto Scaling starts performing a rolling replacement of the instances. It takes a set of instances out of service, terminates them, and launches a set of instances with the new desired configuration. Then, it waits until the instances pass your health checks and complete warmup before it moves on to replacing other instances.
    >
    > After a certain percentage of the group is replaced, a checkpoint is reached. Whenever there is a checkpoint, Amazon EC2 Auto Scaling temporarily stops replacing instances, sends a notification, and waits for the amount of time you specified before continuing. After you receive the notification, you can verify that your new instances are working as expected.
    >
    > After the instance refresh succeeds, the Auto Scaling group settings are automatically updated with the configuration that you specified at the start of the operation.

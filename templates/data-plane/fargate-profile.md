#### Fargate Profile

- ℹ️ [Fargate pod patching](https://docs.aws.amazon.com/eks/latest/userguide/fargate-pod-patching.html)

Note: Fargate profiles are immutable and therefore cannot be changed. However, you can create a new, updated profile to replace an existing profile, and then delete the original. Adding the Kubernetes version to your Fargate profile names will allow you to have one profile name mapped to each version to facilitate upgrades across versions without name conflicts.

1. Create a new Fargate profile(s) with the desired Kubernetes version in the profile name

    ```sh
    aws eks create-fargate-profile --cluster-name <CLUSTER-NAME> \
      --fargate-profile-name <FARGATE-PROFILE-NAME>-{{ target_version }} --pod-execution-role-arn <POD-EXECUTION-ROLE-ARN>
    ```

⚠️ Amazon EKS uses the [Eviction API](https://kubernetes.io/docs/concepts/scheduling-eviction/api-eviction/) to safely drain the pod while respecting the pod disruption budgets that you set for the application(s).

⚠️ To limit the number of pods that are down at one time when pods are patched, you can set pod disruption budgets (PDBs). You can use PDBs to define minimum availability based on the requirements of each of your applications while still allowing updates to occur. For more information, see [Specifying a Disruption Budget for your Application](To limit the number of pods that are down at one time when pods are patched, you can set pod disruption budgets (PDBs). You can use PDBs to define minimum availability based on the requirements of each of your applications while still allowing updates to occur. For more information, see Specifying a Disruption Budget for your Application in the Kubernetes Documentation.) in the Kubernetes Documentation.
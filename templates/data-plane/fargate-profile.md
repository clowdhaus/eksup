#### Fargate Profile

Note: Fargate profiles can't be changed. However, you can create a new, updated profile to replace an existing profile, and then delete the original. It is recommended to

1. Create a new Fargate profile(s) with the desired Kubernetes version in the profile name

```sh
aws eks create-fargate-profile --region <REGION> --cluster-name <CLUSTER-NAME> --fargate-profile-name <FARGATE-PROFILE-NAME>-{{ target_version }} --pod-execution-role-arn <POD-EXECUTION-ROLE-ARN>
```

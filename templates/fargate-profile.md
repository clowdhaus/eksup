### Fargate Profile

- ℹ️ [Fargate pod patching](https://docs.aws.amazon.com/eks/latest/userguide/fargate-pod-patching.html)

Note: Fargate profiles are immutable and therefore cannot be changed. However, you can create a new, updated profile to replace an existing profile, and then delete the original. Adding the Kubernetes version to your Fargate profile names will allow you to have one profile name mapped to each version to facilitate upgrades across versions without name conflicts.

#### Upgrade

- Ensure PDBs set
- Ensure a profile in more than one availability zone (spread across all AZs is preferred)
- Use a mutating webhook to inject `nodeSelector: failure-domain.beta.kubernetes.io/zone: <AZ>` into pods created to distribute across the AZs. (EKS Fargate does not natively do this today - see https://github.com/aws/containers-roadmap/issues/824)
- You cannot set the version of a profile; it is pulled from the control plane version. Once the control plane has been updated, any new virtual nodes created will use the latest patch version for the associated control plane version. This means the virtual nodes will need to be rolled to update
- `kubectl drain <VIRTUAL_NODE> --delete-emptydir-data` will respect PDBs and drain the pod and delete the virtual node
  - How to do this at scale in a rolling fashion?

⚠️ Amazon EKS uses the [Eviction API](https://kubernetes.io/docs/concepts/scheduling-eviction/api-eviction/) to safely drain the pod while respecting the pod disruption budgets that you set for the application(s).

⚠️ To limit the number of pods that are down at one time when pods are patched, you can set pod disruption budgets (PDBs). You can use PDBs to define minimum availability based on the requirements of each of your applications while still allowing updates to occur. For more information, see [Specifying a Disruption Budget for your Application](To limit the number of pods that are down at one time when pods are patched, you can set pod disruption budgets (PDBs). You can use PDBs to define minimum availability based on the requirements of each of your applications while still allowing updates to occur. For more information, see Specifying a Disruption Budget for your Application in the Kubernetes Documentation.) in the Kubernetes Documentation.

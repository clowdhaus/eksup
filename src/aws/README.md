# AWS

Given a cluster name (and optional region):

1. Get the control plane subnets
  a. Get the number of free IPs in each subnet and compare >= 5
2. Get the data plane subnets
  a. Get the number of free IPs in each subnet and show a percentage of total IPs available

## Data Collection for Playbook

1. Get the EKS managed node group(s) and Fargate Profile(s) associated with the cluster
  - For each EKS managed node group, check if any have pending updates (launch template version)
2. Query the Auto Scaling groups for tags that map the ASG to the cluster -> these are self-managed node groups
  - For each self-managed node group, check if any have pending updates (launch template version)
3. Get list of EKS addons and their current versions
  - Check if there are updates to the addons for the current version - do these first
  - Report on the version that is the default for the target version
3. Get the control plane version for comparison against data plane (node versions will be queried from K8s)
4. ? Do we want to query the EC2s for the AMI and check each one to see if custom or EKS AL2 ? Is this valuable?

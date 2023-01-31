# EKS Cluster Upgrade Guidance

## Why Is This Needed

Kubernetes releases a new version [approximately every 4 months](https://kubernetes.io/releases/release/). Each minor version is supported for 12 months after it's first released by the Kubernetes community, and Amazon EKS supports a Kubernetes version for 14 months once made available. In line with the Kubernetes community support for versions, Amazon EKS is committed to supporting at least four versions of Kubernetes at any given time. This means that Amazon EKS users need to be prepared to upgrade their cluster version(s) at least once a year. However, there are a number of factors that can make each upgrade different and unique that users will need to evaluate prior to each upgrade. Factors that can change between each upgrade cycle include:

- New team members who are inexperienced with the upgrade process, and/or prior team members who have experience in cluster upgrades are no longer on the team
- Different Kubernetes APIs are marked as deprecated or removed from the next release
- Kubernetes resources that were previously provided by Kubernetes "in-tree" are now provided as external resources (i.e - moving Kubernetes in-tree cloud provider code out to their respective standalone projects such as ALB ingress controller "in-tree" to the external ALB load balancer controller)
- Various changes and deprecations in the components used by Kubernetes (i.e - moving from `kube-dns` to `CoreDNS`, moving from Docker engine to `containerd` for container runtime, dropping support for `dockershim`, etc.)
- Changes in your applications, your architecture, or the amount of traffic your clusters are handling. Over time, the number of available IPs for the cluster resources may shrink, stateful workloads may have been added to the cluster, etc., and these factors can influence the upgrade process.

### What It Is

`eksup` is a CLI that helps users prepare for a cluster upgrade - providing users as much relevant information as possible for their upgrade.

`eksup` gives users the ability to analyze their cluster(s) against the next version of Kubernetes, highlighting any findings that may affect the upgrade process. In addition, `eksup` has the ability to generate a playbook tailored to the cluster analyzed that provides the process for upgrading the cluster including the findings that require remediation. The playbook output allows users to edit the upgrade steps to suit their cluster configuration and business requirements plus capture any specific learnings during the upgrade process. Since most users typically perform upgrades on nonproduction clusters first, any additional steps or call-outs that are discovered during the upgrade process can be captured and used to improve the upgrade process for their production clusters. Users are encouraged to save their playbooks as historical artifacts for future reference to ensure that with each cycle, the team has a better understanding of the upgrade process and more confidence in swiftly working through cluster upgrades before their Kubernetes version support expires.

### What It Is NOT

- `eksup` is not a tool that will perform the cluster upgrade. It is assumed that clusters are generally created using an infrastructure as code approach through tools such as Terraform, `eksctl`, or CloudFormation. Therefore, users are encouraged to use those tools to perform the upgrade to avoid any resource definition conflicts.
- It does not perform any modifications on the resources it identifies as needing, or recommending, changes. Again, following the approach of infrastructure as code, users are encouraged to make these changes through their normal change control process at the appropriate time in the upgrade process.
  - In the future, `eksup` may provide functionality to help in converting a Kubernetes manifest definition from one API version to the next. However, this will occur on the users local filesystem and not against a live cluster. `eksup` will always operate from the perspective of infrastructure as code; any feature requests that support this tenant are encouraged.

## Commands

```sh linenums="1"
A CLI to aid in upgrading Amazon EKS clusters

Usage: eksup <COMMAND>

Commands:
  analyze  Analyze an Amazon EKS cluster for potential upgrade issues
  create   Create artifacts using the analysis data
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Analyze

Analyze cluster for any potential issues to remediate prior to upgrade.

```sh linenums="1"
Analyze an Amazon EKS cluster for potential upgrade issues

Usage: eksup analyze [OPTIONS] --cluster <CLUSTER>

Options:
  -c, --cluster <CLUSTER>
          The name of the cluster to analyze

  -r, --region <REGION>
          The AWS region where the cluster is provisioned

  -f, --format <FORMAT>
          [default: text]

          Possible values:
          - json: JSON format used for logging or writing to a *.json file
          - text: Text format used for writing to stdout

  -o, --output <OUTPUT>
          Write to file instead of stdout

      --ignore-recommended
          Exclude recommendations from the output

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

Show result as plaintext via stdout:

``` sh linenums="1"
eksup analyze --cluster <cluster> --region <region>
```

Show result as JSON via stdout:

```sh linenums="1"
eksup analyze --cluster <cluster> --region <region> --format json
```

Save result as plaintext to file:

```sh linenums="1"
eksup analyze --cluster <cluster> --region <region> --output analysis.txt
```

Save result as JSON to S3, ignoring recommendations:

```sh linenums="1"
eksup analyze --cluster <cluster> --region <region> \
  --format json --output s3://<bucket>/<filename> --ignore-recommended
```

### Create

Create a playbook with analysis findings to guide users through pre-upgrade, upgrade, and post-upgrade process.

This CLI produces a cluster upgrade playbook that attempts to:

- Educate users on the overall process of upgrading an Amazon EKS cluster (order of operations, which parts AWS manages and which parts are the user's responsibility, etc.)
- Provide one approach as the basis for upgrading a cluster that users can modify/customize to suit their cluster configuration/architecture and business requirements
- Provide recommendations on what to check for and precautions to consider before upgrading, how to perform the cluster upgrade, and considerations for configuring your cluster and/or applications to minimize risk and disruption during the upgrade process

```sh linenums="1"
Create a playbook for upgrading an Amazon EKS cluster

Usage: eksup create playbook [OPTIONS] --cluster <CLUSTER>

Options:
  -c, --cluster <CLUSTER>    The name of the cluster to analyze
  -r, --region <REGION>      The AWS region where the cluster is provisioned
  -f, --filename <FILENAME>  Name of the playbook saved locally
      --ignore-recommended   Exclude recommendations from the output
  -h, --help                 Print help
  -V, --version              Print version
```

Create playbook and save locally:

```sh linenums="1"
eksup create playbook --cluster <cluster> --region <region>
```

Create playbook and save locally, ignoring recommendations:

```sh linenums="1"
eksup create playbook --cluster <cluster> --region <region> --ignore-recommended
```

## Checks

Please refer to [symbol table](https://github.com/clowdhaus/eksup/blob/main/docs/getting-started.md#symbol-table).

### Amazon EKS Checks

| Type |  Code | Description |
| :--: | :---: | :---------- |
| ❌ | `EKS001` | At least 5 available IPs for the control plane to upgrade; required for cross account ENI creation |
| ❌ | `EKS002` | EKS addon(s) are compatible with the next Kubernetes version |
| ❌ | `EKS003` | No health issues reported for the EKS cluster (control plane) |
| ❌ | `EKS004` | No health issues reported for the EKS managed node groups. There aren't any available health statuses available from the AWS API for self-managed node groups or Fargate profiles at this time |
| ❌ | `EKS005` | No health issues reported for the EKS addons |
| ⚠️ | `EKS006` | EKS managed node group(s) are using latest launch template version; no pending updates |
| ⚠️ | `EKS007` | Self-managed node group(s) are using latest launch template version; no pending updates |

### Kubernetes Checks

| Type |  Code | Description |
| :--: | :---: | :---------- |
| ❌ | `K8S001` | The control plane version matches the version used by the data plane |

### AWS Checks

| Type |  Code | Description |
| :--: | :---: | :---------- |
| ⚠️ | `AWS001` | Sufficient available IPs for the nodes to support the surge, in-place rolling upgrade. Irrespective of Kubernetes, each EC2 instance |
| ⚠️ | `AWS002` | Sufficient available IPs for the pods to support the surge, in-place rolling upgrade. This check is used when custom networking is enabled since the IPs used by pods are coming from subnets different from those used by the EC2 instances themselves |

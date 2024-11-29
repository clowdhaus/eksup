```
A CLI to aid in upgrading Amazon EKS clusters

Usage: eksup [OPTIONS] <COMMAND>

Commands:
  analyze  Analyze an Amazon EKS cluster for potential upgrade issues
  create   Create artifacts using the analysis data
  help     Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help
  -V, --version     Print version
```

### Analyze

Analyze cluster for any potential issues to remediate prior to upgrade.

```
Analyze an Amazon EKS cluster for potential upgrade issues

Usage: eksup analyze [OPTIONS] --cluster <CLUSTER>

Options:
  -c, --cluster <CLUSTER>
          The name of the cluster to analyze

  -r, --region <REGION>
          The AWS region where the cluster is provisioned

  -p, --profile <PROFILE>
          The AWS profile to use to access the cluster

  -v, --verbose...
          Increase logging verbosity

  -f, --format <FORMAT>
          [default: text]

          Possible values:
          - json: JSON format used for logging or writing to a *.json file
          - text: Text format used for writing to stdout

  -q, --quiet...
          Decrease logging verbosity

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

!!! info "See [`examples/test-mixed_v1.24_upgrade.md`](https://github.com/clowdhaus/eksup/blob/main/examples/test-mixed_v1.24_upgrade.md) for an example of a playbook created with `eksup`."

This CLI produces a cluster upgrade playbook that attempts to:

- Educate users on the overall process of upgrading an Amazon EKS cluster (order of operations, which parts AWS manages and which parts are the user's responsibility, etc.)
- Provide one approach as the basis for upgrading a cluster that users can modify/customize to suit their cluster configuration/architecture and business requirements
- Provide recommendations on what to check for and precautions to consider before upgrading, how to perform the cluster upgrade, and considerations for configuring your cluster and/or applications to minimize risk and disruption during the upgrade process

```
Create a playbook for upgrading an Amazon EKS cluster

Usage: eksup create playbook [OPTIONS] --cluster <CLUSTER>

Options:
  -c, --cluster <CLUSTER>    The name of the cluster to analyze
  -r, --region <REGION>      The AWS region where the cluster is provisioned
  -p, --profile <PROFILE>    The AWS profile to use to access the cluster
  -v, --verbose...           Increase logging verbosity
  -f, --filename <FILENAME>  Name of the playbook saved locally
  -q, --quiet...             Decrease logging verbosity
  -h, --help                 Print help
  -V, --version              Print version
```

Create playbook and save locally:

```sh linenums="1"
eksup create playbook --cluster <cluster> --region <region>
```

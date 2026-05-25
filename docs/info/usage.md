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
          Possible values:
          - json: JSON format used for logging or writing to a *.json file
          - text: Text format used for writing to stdout

          [default: text]

  -q, --quiet...
          Decrease logging verbosity

  -o, --output <OUTPUT>
          Write to file instead of stdout

  -t, --target-version <TARGET_VERSION>
          Target Kubernetes version for the upgrade (e.g. "1.34"). Defaults to current + 1

      --ignore-recommended
          Exclude recommendations from the output

      --config <CONFIG>
          Path to an eksup configuration file (default: .eksup.yaml in cwd)

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
  -c, --cluster <CLUSTER>
          The name of the cluster to analyze
  -r, --region <REGION>
          The AWS region where the cluster is provisioned
  -p, --profile <PROFILE>
          The AWS profile to use to access the cluster
  -v, --verbose...
          Increase logging verbosity
  -f, --filename <FILENAME>
          Name of the playbook saved locally
  -q, --quiet...
          Decrease logging verbosity
  -t, --target-version <TARGET_VERSION>
          Target Kubernetes version for the upgrade (e.g. "1.34"). Defaults to current + 1
      --ignore-recommended
          Exclude recommendations from the output
      --config <CONFIG>
          Path to an eksup configuration file (default: .eksup.yaml in cwd)
  -h, --help
          Print help
  -V, --version
          Print version
```

Create playbook and save locally:

```sh linenums="1"
eksup create playbook --cluster <cluster> --region <region>
```

### Configuration

`eksup` supports an optional configuration file (`.eksup.yaml`) for customizing check behavior. By default, `eksup` looks for `.eksup.yaml` in the current working directory. You can specify a custom path with the `--config` flag:

```sh
eksup analyze --cluster my-cluster --config /path/to/config.yaml
```

#### Configuration Format

```yaml
# .eksup.yaml — full example
checks:
  all:                          # applies to every workload check
    ignore:
      - name: "*"
        namespace: "*-dev*"     # skip all dev namespaces

  K8S002:
    min_replicas: 3             # default minimum
    ignore:                     # globs supported
      - name: "preview-*"
        namespace: "preview"
    overrides:                  # globs supported here too
      - name: "stateful-*"
        namespace: "prod"
        min_replicas: 5

  K8S003: { ignore: [{ name: "*-batch", namespace: "*" }] }
  K8S004: { ignore: [{ name: "singleton-*", namespace: "default" }] }
  K8S005: { ignore: [{ name: "*-logger*", namespace: "*monitoring*" }] }
  # ... K8S006, K8S007, K8S008, K8S013 all use the same `ignore:` shape
```

- `K8S002.min_replicas`: Global minimum replica threshold (default: 2). Must be >= 1.
- `K8S002.overrides`: Per-resource minimum replica threshold (globs supported). Overrides the global default.
- `ignore` takes precedence over `overrides` when both match the same resource.
- Unknown fields in the configuration file are rejected with an error.

#### The `checks.all` block

`checks.all.ignore` applies across **every** workload check. Selectors listed here are merged with each check's own `ignore` list, so you can suppress noisy namespaces or naming patterns in one place instead of repeating the same entry under every code.

#### Glob syntax

`name` and `namespace` selectors support standard glob syntax:

- `*` — matches any sequence of characters (including empty)
- `?` — matches exactly one character
- `[abc]` / `[a-z]` — character classes
- `{a,b,c}` — brace expansion (alternation)

Literal strings still work and continue to behave as exact matches — existing configs without globs remain valid.

#### Workload checks that support `ignore`

The following workload checks accept the `ignore:` shape shown above (each rule is matched against the resource's `name` + `namespace`):

- `K8S002` — minimum replicas
- `K8S003` — minimum ready seconds
- `K8S004` — PodDisruptionBudget present
- `K8S005` — pod topology spread / anti-affinity
- `K8S006` — readiness probe configured
- `K8S007` — termination grace period
- `K8S008` — Docker socket mount
- `K8S013` — `ingress-nginx` controller retirement

#### Cluster-level checks (no `ignore` support)

Cluster-level checks do not support `ignore` because their findings have no resource `name` + `namespace` to match against:

- `AWS001` – `AWS005` — AWS-side checks
- `EKS001` – `EKS010` — EKS-side checks
- `K8S001` — Kubernetes version skew
- `K8S009` — Pod Security Policies
- `K8S010` — EBS CSI driver
- `K8S011` — kube-proxy / kubelet version skew
- `K8S012` — kube-proxy IPVS mode (singleton DaemonSet; finding has no per-resource fields)

The right control for a cluster-level check is "disable the check entirely" — a separate, future feature.

#### `--show-suppressed` flag

Both `eksup analyze` and `eksup create playbook` accept `--show-suppressed`. By default, suppressed findings are hidden and a single-line summary (e.g. `3 findings suppressed by .eksup.yaml (use --show-suppressed to view)`) is printed. With the flag set, suppressed findings are rendered inline under a `Suppressed by .eksup.yaml (N):` section so you can audit exactly what your config is filtering out.

#### JSON output: `suppressed` key

The JSON output (`--format json`) always includes a top-level `suppressed:` key alongside the regular findings, regardless of `--show-suppressed`. Programmatic consumers can inspect every suppressed finding without re-running the analysis. The `--show-suppressed` flag only affects human-readable stdout/playbook rendering.

# Design

## Goals

There is one primary goal for `eksup`:

!!! success "Empower users to routinely upgrade their EKS cluster(s) while avoiding downtime and/or disruption."

To aid in that goal, the following are supporting goals or tenants:

1. Intended for use on Amazon EKS clusters; there are no guarantees that this CLI will or will not work on other Kubernetes clusters at this time. The CLI will focus on EKS to avoid the need to support multiple Kubernetes distributions and their associated cloud controller resources and instead focus on the aspects that are specific to EKS and how it manages the Kubernetes experience for users. The CLI should offer native support for:
      - Amazon EKS, Amazon EKS on Outposts (local and extended clusters), and [Not yet supported] EKS-A (EKS Anywhere)
      - Amazon EKS managed node groups, self-managed node groups, and Amazon EKS Fargate profiles
      - EKS addons
2. It is designed to produce the least amount of load on the API Server when discovering and analyzing cluster resources. However, this can have some tradeoffs in terms of accuracy and completeness of the information provided to the user
3. It should provide as much relevant information as possible to the user regarding the state of the cluster prior to upgrading. This includes scoping the information provided to the user to only that which is relevant for upgrading from their current Kubernetes version to the intended target Kubernetes version.
4. It should support the following use cases:
      - A one-off analysis to create a report of the cluster state prior to upgrading along with steps to take to upgrade the cluster (i.e. - analyze the cluster and generate an upgrade playbook).
      - A one-off analysis to report on the state of the cluster for potential issues and/or recommendations prior to upgrading. This is generally a CLI invocation that prints the analysis to the console (stdout) and exits.
      - Continuous analysis of the cluster state for potential issues and/or recommendations that runs from within the cluster that is being reported on. Results can be sent to stdout where they can be picked up from a log aggregator or sent to a remote location, such as Amazon S3, where they can be analyzed and acted upon. This process supports n-number of clusters across m-number of accounts to better aid in multi-cluster management as well as alerting to ensure enough advance notice is given for users to prepare and schedule the pending upgrade before end of support is reached.

## Architecture

### High Level Diagram

![High level diagram](https://raw.githubusercontent.com/clowdhaus/eksup/main/docs/imgs/checks.png)

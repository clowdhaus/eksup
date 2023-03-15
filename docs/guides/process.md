# Process

This guide is intended to highlight the process for planning and preparing for routine cluster upgrades. It includes the timing of various activities, testing and validation strategies, as well as helpful tips on reducing the amount of time and effort required to perform upgrades.

## Planning

It is important to plan for upgrades in advance to allow for ample time to test, validate, and perform the upgrades. The target for users is to aim to be on at least the 2nd latest version of Kubernetes provided by Amazon EKS. If the current latest version supported by Amazon EKS is 1.25, users should either be on 1.24 or in the process of upgrading to 1.24. There is no harm in trying to stay on the latest version supported by Amazon EKS, but you may run into issues with 3rd party/OSS software that you run on top of Kubernetes depending on how quickly the 3rd party/OSS software is updated to support the latest version.

## Testing & Validation

## Helpful Tips


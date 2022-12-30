# r8s

There are three methods that are used by other tools in the ecosystem today to find resources that are utilizing an API version that is no longer served by the API server. They are:
   1. Retrieving the `kubectl.kubernetes.io/last-applied-configuration` and using the API version in that annotation to determine if there is a potential for conflict after an upgrade. This method is highly problematic in that the annotation cannot be fully relied on since not all creation methods will update this annotation and therefore not show all the potential API version conflicts. This method, if used, should be treated as informative but not definitive.
   2. Retrieving the versions listed in the secret(s) created by Helm. This method relies on the assumption that Helm is the authority on resource creation which is not always the case. For example, operators may use client SDKs to create resources which will not show up in the Helm secrets or users may have used other means to provison resources (`kubectl`, SDKs, etc.). There seems to be some issues with this approach as well - namely that a large number of Helm charts/installs will lead to timeouts when checking for deprecated versions, as well as false positives when Helm charts were not properly cleaned up when removed from the cluter (i.e. - upgrading from Helm v2 to v3, resource finalizers, etc.).
   3. The last, and most thorough (though still not definitive) method is to interrogate the manifests used to provision resources on the cluster. If users follow a GitOps process where all resources provisioned on the cluster are codified in manifests/conifugarions/charts stored in a git repository, the it is possible to search through these manifests for API versions that are marked as deprecated and/or removed. The only caveat that prevents this method from being definitive is the fact that what is deployed from the manifests potentially may go on to create additional resources that are not tracked in the manifests.

## Reference [sig-architecture/api_changes.md](https://github.com/kubernetes/community/blob/master/contributors/devel/sig-architecture/api_changes.md)

The Kubernetes API has two major components - the internal structures and the versioned APIs. The versioned APIs are intended to be stable,  while the internal structures are implemented to best reflect the needs of the Kubernetes code itself. Every versioned API can be converted to the internal form (and vice-versa), but versioned APIs do not convert to other versioned APIs directly. While all of the Kubernetes code operates on the internal structures, they are always converted to a versioned form before being written to storage (disk or etcd) or being sent over a wire. Clients should consume and operate on the versioned APIs exclusively.

To demonstrate the general process, here is a (hypothetical) example:

   1. A user POSTs a `Pod` object to `/api/v7beta1/...`
   2. The JSON is unmarshalled into a `v7beta1.Pod` structure
   3. Default values are applied to the `v7beta1.Pod`
   4. The `v7beta1.Pod` is converted to an `api.Pod` structure
   5. The `api.Pod` is validated, and any errors are returned to the user
   6. The `api.Pod` is converted to a `v6.Pod` (because v6 is the latest stable version)
   7. The `v6.Pod` is marshalled into JSON and written to etcd

Now that we have the `Pod` object stored, a user can GET that object in any supported api version. For example:

   1. A user GETs the `Pod` from `/api/v5/...`
   2. The JSON is read from etcd and unmarshalled into a `v6.Pod` structure
   3. Default values are applied to the `v6.Pod`
   4. The `v6.Pod` is converted to an `api.Pod` structure
   5. The `api.Pod` is converted to a `v5.Pod` structure
   6. The `v5.Pod` is marshalled into JSON and sent to the user

## Notes

- If the `kubectl.kubernetes.io/last-applied-configuration` annotation contains an old API version, it suggests the resource was created using the old API version (i.e. - you will run into trouble if you try to perform the same operation again after the cluster has been upgraded).
- The API version of the returned object will be governed by the client and not representative of what will or will not fail if POSTed after a cluster upgrade.

## Links

- https://github.com/kubernetes/kubernetes/issues/58131#issuecomment-404466779
   - https://github.com/kubernetes/kubernetes/issues/58131
- https://github.com/kubernetes/community/blob/master/contributors/devel/sig-architecture/api-conventions.md
- https://kubernetes.io/docs/reference/using-api/#api-versioning
- https://github.com/kubernetes/client-go
- https://kubernetes.io/docs/reference/using-api/deprecation-policy/

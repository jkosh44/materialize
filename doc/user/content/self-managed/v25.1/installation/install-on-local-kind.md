---
title: "Install locally on kind"
description: ""
aliases:
  - /self-hosted/install-on-local-kind/
  - /self-managed/installation/install-on-local-kind/
suppress_breadcrumb: true
---

The following tutorial deploys self-managed Materialize onto a local
[`kind`](https://kind.sigs.k8s.io/) cluster. Self-managed Materialize requires:

{{% self-managed/requirements-list %}}

The tutorial uses a local `kind` cluster and deploys the following components:

- Materialize Operator using Helm into your local `kind` cluster.
- MinIO object storage as the blob storage for your Materialize.
- PostgreSQL database as the metadata database for your Materialize.
- Materialize as a containerized application into your local `kind` cluster.

{{< important >}}

For testing purposes only.

{{< /important >}}

## Prerequisites

### kind

Install [`kind`](https://kind.sigs.k8s.io/docs/user/quick-start/).

### Docker

Install [`Docker`](https://docs.docker.com/get-started/get-docker/).

### Helm 3.2.0+

If you don't have Helm version 3.2.0+ installed, refer to the [Helm
documentation](https://helm.sh/docs/intro/install/).

### `kubectl`

This tutorial uses `kubectl`. To install, refer to the [`kubectl`
documentationq](https://kubernetes.io/docs/tasks/tools/).

For help with `kubectl` commands, see [kubectl Quick
reference](https://kubernetes.io/docs/reference/kubectl/quick-reference/).

### Materialize repo

The following instructions assume that you are installing from the [Materialize
repo](https://github.com/MaterializeInc/materialize).

{{< important >}}

Check out the {{% self-managed/latest_version %}} tag.

{{< /important >}}

## Installation

1. Start Docker if it is not already running.

1. Open a Terminal window.

1. Create a kind cluster.

   ```shell
   kind create cluster
   ```

1. Install the Materialize Helm chart using the files provided in the
   Materialize repo.

   1. Go to the Materialize repo directory.

   1. Install the Materialize operator with the release name
      `my-materialize-operator` into the `materialize` namespace:

      ```shell
      helm install my-materialize-operator \
         -f misc/helm-charts/operator/values.yaml misc/helm-charts/operator \
         --namespace materialize --create-namespace
      ```

   1. Verify the installation and check the status:

      ```shell
      kubectl get all -n materialize
      ```

      Wait for the components to be in the `Running` state:

      ```none
      NAME                                           READY   STATUS              RESTARTS   AGE
      pod/my-materialize-operator-776b98455b-w9kkl   0/1     ContainerCreating   0          6s

      NAME                                      READY   UP-TO-DATE   AVAILABLE   AGE
      deployment.apps/my-materialize-operator   0/1     1            0           6s

      NAME                                                 DESIRED   CURRENT   READY   AGE
      replicaset.apps/my-materialize-operator-776b98455b   1         1         0       6s
      ```

      If you run into an error during deployment, refer to the
      [Troubleshooting](/self-managed/v25.1/installation/troubleshooting) guide.

1. Install PostgreSQL and minIO.

    1. Go to the Materialize repo directory.

    1. Use the provided `postgres.yaml` file to install PostgreSQL as the
       metadata database:

        ```shell
        kubectl apply -f misc/helm-charts/testing/postgres.yaml
        ```

    1. Use the provided `minio.yaml` file to install minIO as the blob storage:

        ```shell
        kubectl apply -f misc/helm-charts/testing/minio.yaml
        ```

1. Optional. Install the following metrics service for certain system metrics
   but not required.

   ```shell
   kubectl apply -f https://github.com/kubernetes-sigs/metrics-server/releases/latest/download/components.yaml
   ```

1. Install Materialize into a new `materialize-environment` namespace:

   1. Go to the Materialize repo directory.

   1. Use the provided `materialize.yaml` file to create the
      `materialize-environment` namespace and install Materialize:

      ```shell
      kubectl apply -f misc/helm-charts/testing/materialize.yaml
      ```

    1. Verify the installation and check the status:

       ```shell
       kubectl get all -n materialize-environment
       ```

       Wait for the components to be in the `Running` state.

       ```none
       NAME                                             READY   STATUS    RESTARTS   AGE
       pod/mzlvmx9h6dpx-balancerd-f5c689b95-kjtzf       1/1     Running   0          45s
       pod/mzlvmx9h6dpx-cluster-s1-replica-s1-gen-1-0   1/1     Running   0          51s
       pod/mzlvmx9h6dpx-cluster-s2-replica-s2-gen-1-0   1/1     Running   0          51s
       pod/mzlvmx9h6dpx-cluster-s3-replica-s3-gen-1-0   1/1     Running   0          51s
       pod/mzlvmx9h6dpx-cluster-u1-replica-u1-gen-1-0   1/1     Running   0          51s
       pod/mzlvmx9h6dpx-console-6b746b7d57-p24n4        1/1     Running   0          32s
       pod/mzlvmx9h6dpx-console-6b746b7d57-qjs4p        1/1     Running   0          32s
       pod/mzlvmx9h6dpx-environmentd-1-0                1/1     Running   0          60s

       NAME                                               TYPE        CLUSTER-IP   EXTERNAL-IP   PORT(S)                                        AGE
       service/mzlvmx9h6dpx-balancerd                     ClusterIP   None         <none>        6876/TCP,6875 TCP                              45s
       service/mzlvmx9h6dpx-cluster-s1-replica-s1-gen-1   ClusterIP   None         <none>        2100/TCP,2103/TCP,2101/TCP,2102/TCP,6878 TCP   51s
       service/mzlvmx9h6dpx-cluster-s2-replica-s2-gen-1   ClusterIP   None         <none>        2100/TCP,2103/TCP,2101/TCP,2102/TCP,6878 TCP   51s
       service/mzlvmx9h6dpx-cluster-s3-replica-s3-gen-1   ClusterIP   None         <none>        2100/TCP,2103/TCP,2101/TCP,2102/TCP,6878 TCP   51s
       service/mzlvmx9h6dpx-cluster-u1-replica-u1-gen-1   ClusterIP   None         <none>        2100/TCP,2103/TCP,2101/TCP,2102/TCP,6878 TCP   51s
       service/mzlvmx9h6dpx-console                       ClusterIP   None         <none>        8080 TCP                                       32s
       service/mzlvmx9h6dpx-environmentd                  ClusterIP   None         <none>        6875/TCP,6876/TCP,6877/TCP,6878 TCP            45s
       service/mzlvmx9h6dpx-environmentd-1                ClusterIP   None         <none>        6875/TCP,6876/TCP,6877/TCP,6878 TCP            60s
       service/mzlvmx9h6dpx-persist-pubsub-1              ClusterIP   None         <none>        6879 TCP                                       60s

       NAME                                     READY   UP-TO-DATE   AVAILABLE   AGE
       deployment.apps/mzlvmx9h6dpx-balancerd   1/1     1            1           45s
       deployment.apps/mzlvmx9h6dpx-console     2/2     2            2           32s

       NAME                                               DESIRED   CURRENT   READY   AGE
       replicaset.apps/mzlvmx9h6dpx-balancerd-f5c689b95   1         1         1       45s
       replicaset.apps/mzlvmx9h6dpx-console-6b746b7d57    2         2         2       32s

       NAME                                                        READY   AGE
       statefulset.apps/mzlvmx9h6dpx-cluster-s1-replica-s1-gen-1   1/1     51s
       statefulset.apps/mzlvmx9h6dpx-cluster-s2-replica-s2-gen-1   1/1     51s
       statefulset.apps/mzlvmx9h6dpx-cluster-s3-replica-s3-gen-1   1/1     51s
       statefulset.apps/mzlvmx9h6dpx-cluster-u1-replica-u1-gen-1   1/1     51s
       statefulset.apps/mzlvmx9h6dpx-environmentd-1                1/1     60s
       ```

       If you run into an error during deployment, refer to the
       [Troubleshooting](/self-managed/troubleshooting) guide.

1. Open the Materialize console in your browser:

   1. From the previous `kubectl` output, find the Materialize console service.

      ```none
      NAME                           TYPE        CLUSTER-IP   EXTERNAL-IP   PORT(S)    AGE
      service/mzlvmx9h6dpx-console   ClusterIP   None         <none>        8080 TCP   32s
      ```

   1. Forward the Materialize console service to your local machine (substitute
      your service name for `mzlvmx9h6dpx-console`):

      ```shell
      while true;
      do kubectl port-forward svc/mzlvmx9h6dpx-console 8080:8080 -n materialize-environment 2>&1 |
      grep -q "portforward.go" && echo "Restarting port forwarding due to an error." || break;
      done;
      ```
      {{< note >}}
      Due to a [known Kubernetes issue](https://github.com/kubernetes/kubernetes/issues/78446),
      interrupted long-running requests through a standard port-forward cause the port forward to hang. The command above
      automatically restarts the port forwarding if an error occurs, ensuring a more stable
      connection. It detects failures by monitoring for "portforward.go" error messages.
      {{< /note >}}

   1. Open a browser and navigate to
      [http://localhost:8080](http://localhost:8080).

      ![Image of  self-managed Materialize console running on local kind](/images/self-managed/self-managed-console-kind.png)

## See also

- [Materialize Kubernetes Operator Helm Chart](/self-managed/)
- [Materialize Operator Configuration](/self-managed/configuration/)
- [Troubleshooting](/self-managed/troubleshooting/)
- [Operational guidelines](/self-managed/operational-guidelines/)
- [Installation](/self-managed/installation/)
- [Upgrading](/self-managed/upgrading/)

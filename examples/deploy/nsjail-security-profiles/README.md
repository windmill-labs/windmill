Security profiles for nsjail without a privileged container
============================================================

[nsjail sandboxing](https://www.windmill.dev/docs/advanced/security_isolation#nsjail-sandboxing) builds each job its own PID and mount namespaces. A container runtime's default seccomp and AppArmor profiles block the system calls and mounts this takes, which is why nsjail is usually run with `privileged: true` or with both profiles set to `Unconfined`.

The two profiles here are the runtime defaults with only what nsjail needs added, so a worker can run nsjail in a non-privileged container that keeps a seccomp filter and an AppArmor profile. See [Running nsjail without privileged: true](https://www.windmill.dev/docs/advanced/security_isolation#running-nsjail-without-privileged-true) for the full guide.

## Files

| File | Purpose |
| --- | --- |
| `windmill-nsjail.seccomp.json` | Docker's default seccomp profile plus `clone` with namespace flags, `mount`, `umount2`, `pivot_root` and `sethostname` |
| `windmill-nsjail.apparmor` | The [runtime default AppArmor profile](https://github.com/moby/profiles/blob/main/apparmor/template.go) with `deny mount` replaced by `mount`, `umount` and `pivot_root` rules; every other rule, including the denied socket families, is kept |
| `kubernetes-user-namespaces.yaml` | Worker pod settings with no added capability (`hostUsers: false`, `procMount: Unmasked`) |
| `kubernetes-sys-admin.yaml` | Worker pod settings for clusters without user namespaces (`SYS_ADMIN`) |
| `generate-seccomp.py` | Rebuilds the seccomp profile from a newer Docker default |

## Installing the profiles on Kubernetes nodes

Both profiles are read from the node, so they have to be present on every node that runs workers.

```bash
# seccomp: relative to the kubelet's seccomp directory
sudo install -D -m 0644 windmill-nsjail.seccomp.json /var/lib/kubelet/seccomp/profiles/windmill-nsjail.json

# AppArmor: only on nodes where AppArmor is enabled
sudo install -m 0644 windmill-nsjail.apparmor /etc/apparmor.d/windmill-nsjail
sudo apparmor_parser -r /etc/apparmor.d/windmill-nsjail
```

A pod that references a profile missing from its node fails to start, so node groups that autoscale need these steps in their bootstrap or in a DaemonSet. On nodes without AppArmor, remove the `appArmorProfile` block from the pod settings.

## Using the profiles with Docker

```bash
sudo install -m 0644 windmill-nsjail.apparmor /etc/apparmor.d/windmill-nsjail
sudo apparmor_parser -r /etc/apparmor.d/windmill-nsjail

docker run \
  --user 1000:1000 --cap-drop ALL \
  --security-opt no-new-privileges \
  --security-opt systempaths=unconfined \
  --security-opt seccomp=windmill-nsjail.seccomp.json \
  --security-opt apparmor=windmill-nsjail \
  ...
```

## Enabling nsjail

The profiles and pod settings only make nsjail able to run. To turn it on, set the **Job isolation** [instance setting](https://www.windmill.dev/docs/advanced/instance_settings) to **Nsjail**. It applies to every worker of the instance, so each of them needs settings under which nsjail can run.

The `DISABLE_NSJAIL=false` environment variable is the fallback: it enables nsjail on the workers that carry it, whatever the instance setting says. Use it to sandbox only some worker groups.

## Which pod settings to use

| | `kubernetes-user-namespaces.yaml` | `kubernetes-sys-admin.yaml` |
| --- | --- | --- |
| Needs | Kubernetes 1.33+ with [user namespaces](https://kubernetes.io/docs/concepts/workloads/pods/user-namespaces/) | Kubernetes 1.30+ (older versions set AppArmor with an annotation instead of `appArmorProfile`) |
| Worker user | uid 1000 | root |
| Added capabilities | none | `SYS_ADMIN`, `SETPCAP`, and optionally `SYS_RESOURCE` |
| Pod Security Standards | `baseline` on 1.35+; 1.33 and 1.34 reject `procMount: Unmasked` unless the `UserNamespacesPodSecurityStandards` feature gate is on | needs an exemption for the added capabilities |

## What the profiles give up

Jobs inherit the seccomp filter, so job code can also call `clone` with namespace flags and `mount`, which the runtime default denies to a container without `SYS_ADMIN`. Where the kernel allows unprivileged user namespaces, a job can therefore create a nested user and mount namespace and mount filesystems inside it. This grants nothing outside that namespace, but it exposes more of the kernel to job code than the runtime default does.

The AppArmor profile no longer denies mounts, so its path rules on `/proc` and `/sys` do not hold against a process that can mount those filesystems elsewhere. Both profiles remain far narrower than `Unconfined` or `privileged: true`.

## Regenerating the seccomp profile

`windmill-nsjail.seccomp.json` is derived from the [default profile of the Moby project](https://github.com/moby/profiles/blob/main/seccomp/default.json) (Apache License 2.0). To rebase it on a newer default:

```bash
curl -fsSLO https://raw.githubusercontent.com/moby/profiles/main/seccomp/default.json
python3 generate-seccomp.py default.json windmill-nsjail.seccomp.json
```

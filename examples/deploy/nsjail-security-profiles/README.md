Security profiles for nsjail without a privileged container
============================================================

[nsjail sandboxing](https://www.windmill.dev/docs/advanced/security_isolation#nsjail-sandboxing) builds each job its own PID and mount namespaces. A container runtime's default seccomp and AppArmor profiles block the system calls and mounts this takes, which is why nsjail is usually run with `privileged: true` or with both profiles set to `Unconfined`.

The two profiles here are the runtime defaults with only what nsjail needs added, so a worker can run nsjail in a non-privileged container that keeps a seccomp filter and an AppArmor profile. See [Running nsjail without privileged: true](https://www.windmill.dev/docs/advanced/security_isolation#running-nsjail-without-privileged-true) for the full guide.

## Files

| File | Purpose |
| --- | --- |
| `windmill-nsjail.seccomp.json` | Docker's default seccomp profile plus `clone` with namespace flags, `mount`, `umount2`, `pivot_root` and `sethostname` |
| `windmill-nsjail.apparmor` | The runtime default AppArmor profile with `deny mount` replaced by `mount`, `umount` and `pivot_root` rules |
| `kubernetes-user-namespaces.yaml` | Worker pod settings with no added capability (`hostUsers: false`, `procMount: Unmasked`) |
| `kubernetes-sys-admin.yaml` | Worker pod settings for clusters without user namespaces (`SYS_ADMIN`) |
| `generate-seccomp.py` | Rebuilds the seccomp profile from a newer Docker default |

## Installing the profiles on Kubernetes nodes

Both profiles are read from the node, so they have to be present on every node that runs workers.

```bash
# seccomp: relative to the kubelet's seccomp directory
sudo install -D -m 0644 windmill-nsjail.seccomp.json /var/lib/kubelet/seccomp/profiles/windmill-nsjail.json

# AppArmor: only on nodes where AppArmor is enabled
sudo apparmor_parser -r windmill-nsjail.apparmor
```

On nodes without AppArmor, remove the `appArmorProfile` block from the pod settings.

## Using the profiles with Docker

```bash
sudo apparmor_parser -r windmill-nsjail.apparmor

docker run \
  --user 1000:1000 --cap-drop ALL \
  --security-opt no-new-privileges \
  --security-opt systempaths=unconfined \
  --security-opt seccomp=windmill-nsjail.seccomp.json \
  --security-opt apparmor=windmill-nsjail \
  -e DISABLE_NSJAIL=false \
  ...
```

## Which pod settings to use

| | `kubernetes-user-namespaces.yaml` | `kubernetes-sys-admin.yaml` |
| --- | --- | --- |
| Needs | Kubernetes 1.33+ with [user namespaces](https://kubernetes.io/docs/concepts/workloads/pods/user-namespaces/) | Any cluster |
| Worker user | uid 1000 | root |
| Added capabilities | none | `SYS_ADMIN`, `SETPCAP` |
| Pod Security Standards | `baseline` on 1.35+; 1.33 and 1.34 reject `procMount: Unmasked` unless the `UserNamespacesPodSecurityStandards` feature gate is on | needs an exemption for the added capabilities |

## Regenerating the seccomp profile

`windmill-nsjail.seccomp.json` is derived from the [default profile of the Moby project](https://github.com/moby/profiles/blob/main/seccomp/default.json) (Apache License 2.0). To rebase it on a newer default:

```bash
curl -fsSLO https://raw.githubusercontent.com/moby/profiles/main/seccomp/default.json
python3 generate-seccomp.py default.json windmill-nsjail.seccomp.json
```

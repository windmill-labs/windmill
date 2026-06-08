// Host-side cache for container images that the cluster pulls.
//
// Cluster containers run inside the minikube VM, and `minikube delete` destroys
// the VM's container runtime — next bringup re-pulls everything (windmill
// alone is ~3.85 GB per node). We work around that with two hooks:
//   - on bringup (after `minikube start`, before `helm install`): for every
//     `.tar` in the host cache dir, `minikube image load <tar>` into the new
//     VM(s).
//   - on teardown (before `minikube delete`): `minikube image save <img> <tar>`
//     for each workload image currently in the cluster, skipping ones already
//     cached.
//
// Net effect: first run pulls and caches; subsequent runs reuse the cache,
// trading a multi-GB network pull for a ~30 s disk-to-VM load per node.
//
// Default cache dir: $XDG_CACHE_HOME/wm-sim/images (~/.cache/wm-sim/images).

const MINIKUBE = Deno.env.get("SIM_MINIKUBE_BIN") ?? "minikube";

// System images we don't cache — they're small, kube-system-only, and minikube
// itself often pre-stages them via its ISO/preload bundle.
const SYSTEM_PREFIXES = [
  "registry.k8s.io/",
  "gcr.io/k8s-minikube/",
  "docker.io/flannel/",
  "k8s.gcr.io/",
  "kindest/",
];

function defaultCacheDir(): string {
  const xdg = Deno.env.get("XDG_CACHE_HOME");
  const home = Deno.env.get("HOME") ?? ".";
  return `${xdg ?? `${home}/.cache`}/wm-sim/images`;
}

// Same env shape the provisioner uses — minikube needs the kvm2 driver dir on
// PATH + LD_LIBRARY_PATH for libvirt.
function minikubeEnv(): Record<string, string> {
  const base = Deno.env.toObject();
  const driverDir = Deno.env.get("SIM_KVM2_DRIVER_DIR");
  const libDir = Deno.env.get("SIM_LIBVIRT_LIB_DIR");
  if (driverDir) base.PATH = `${driverDir}:${base.PATH ?? ""}`;
  if (libDir) {
    base.LD_LIBRARY_PATH = base.LD_LIBRARY_PATH
      ? `${libDir}:${base.LD_LIBRARY_PATH}`
      : libDir;
  }
  return base;
}

async function runMinikube(
  args: string[],
): Promise<{ code: number; stdout: string; stderr: string }> {
  const p = new Deno.Command(MINIKUBE, {
    args,
    env: minikubeEnv(),
    stdout: "piped",
    stderr: "piped",
  });
  const { code, stdout, stderr } = await p.output();
  return {
    code,
    stdout: new TextDecoder().decode(stdout),
    stderr: new TextDecoder().decode(stderr),
  };
}

function tarballName(image: string): string {
  // ghcr.io/windmill-labs/windmill:1.711.0 -> ghcr.io_windmill-labs_windmill_1.711.0.tar
  return image.replace(/[/:@]/g, "_") + ".tar";
}

async function pathExists(p: string): Promise<boolean> {
  try { await Deno.stat(p); return true; } catch { return false; }
}

function isWorkloadImage(image: string): boolean {
  return !SYSTEM_PREFIXES.some((p) => image.startsWith(p));
}

// Load every .tar in cacheDir into the cluster. Best-effort: errors are
// logged but don't abort — a corrupted tarball shouldn't break bringup.
export async function loadCachedImages(
  profile: string,
  cacheDir = defaultCacheDir(),
): Promise<number> {
  if (!(await pathExists(cacheDir))) return 0;
  let loaded = 0;
  for await (const ent of Deno.readDir(cacheDir)) {
    if (!ent.isFile || !ent.name.endsWith(".tar")) continue;
    const path = `${cacheDir}/${ent.name}`;
    console.log(`[cache] loading ${ent.name}`);
    const r = await runMinikube(["-p", profile, "image", "load", path]);
    if (r.code === 0) loaded++;
    else console.warn(`[cache] failed to load ${ent.name}: ${r.stderr.trim().split("\n").pop()}`);
  }
  if (loaded > 0) console.log(`[cache] loaded ${loaded} image(s) from ${cacheDir}`);
  return loaded;
}

// Save each "workload" image in the running cluster to a tarball in cacheDir.
// Skips images already cached. Discovers images via the supplied kubectl
// runner (typically `prov.kubectl`).
export async function saveImagesToCache(
  profile: string,
  kubectl: (args: string[]) => Promise<{ stdout: string; code: number }>,
  cacheDir = defaultCacheDir(),
): Promise<number> {
  await Deno.mkdir(cacheDir, { recursive: true });

  const r = await kubectl([
    "get", "pods", "-A",
    "-o", "jsonpath={range .items[*].spec.containers[*]}{.image}{\"\\n\"}{end}",
  ]);
  if (r.code !== 0) {
    console.warn("[cache] could not list images — skipping save");
    return 0;
  }
  const images = new Set(
    r.stdout.split("\n").map((l) => l.trim()).filter((l) => l && isWorkloadImage(l)),
  );

  let saved = 0;
  for (const image of images) {
    const tarPath = `${cacheDir}/${tarballName(image)}`;
    if (await pathExists(tarPath)) continue;
    console.log(`[cache] saving ${image}`);
    const s = await runMinikube(["-p", profile, "image", "save", image, tarPath]);
    if (s.code === 0) saved++;
    else console.warn(`[cache] failed to save ${image}: ${s.stderr.trim().split("\n").pop()}`);
  }
  if (saved > 0) console.log(`[cache] saved ${saved} new image(s) to ${cacheDir}`);
  return saved;
}

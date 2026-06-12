# minikube's kvm2 driver (docker-machine-driver-kvm2), built from the minikube
# source. Not packaged in nixpkgs; minikube's auto-download won't run on NixOS
# (wrong dynamic linker), so we build it here. Used by the k8s sim path (wm_sim
# / sim/sim.ts) via SIM_KVM2_DRIVER_DIR.
#
# IMPORTANT: build with the *same nixpkgs as the system libvirtd*, otherwise the
# driver links a different glibc than the system-libvirt libs it loads at
# runtime, and you'll see `GLIBC_ABI_DT_X86_64_PLT not found`. The flake passes
# `pkgs = pkgsSim` (fresh nixos-unstable, system-matching) for that reason.
#
# Gotchas baked in:
#   - v1.38.1 ships the driver *library* (pkg/drivers/kvm) but NOT the
#     cmd/drivers/kvm main wrapper (Makefile references it but it's absent),
#     so we synthesise a minimal main mirroring cmd/drivers/hyperkit/main.go.
#     Uses 1.38's internalised plugin path (k8s.io/minikube/pkg/libmachine/...).
#   - Built WITHOUT the `libvirt_dlopen` tag so libvirt is linked (nix sets the
#     rpath). The libvirt-go binding *still* dlopens libvirt.so.0 by soname at
#     connect time, so the caller must set LD_LIBRARY_PATH to a libvirt
#     compatible with the system libvirtd (NixOS has no ld cache).
{ pkgs ? import <nixpkgs> { } }:
pkgs.buildGoModule rec {
  pname = "docker-machine-driver-kvm2";
  version = "1.38.1";

  src = pkgs.fetchFromGitHub {
    owner = "kubernetes";
    repo = "minikube";
    rev = "v${version}";
    hash = "sha256-1unwbu2pJviHXukQKalJLgrkHpjf0sRR2nCm2gKv2VU=";
  };

  vendorHash = "sha256-Oy8cM/foZKC83PxqkJW+o8vVYJhszKxXs9l2eks7FN4=";

  postPatch = ''
    mkdir -p cmd/drivers/kvm
    cat > cmd/drivers/kvm/main.go <<'EOF'
//go:build linux

package main

import (
	"k8s.io/minikube/pkg/drivers/kvm"
	"k8s.io/minikube/pkg/libmachine/drivers/plugin"
)

func main() {
	plugin.RegisterDriver(kvm.NewDriver("", ""))
}
EOF
  '';

  subPackages = [ "cmd/drivers/kvm" ];

  nativeBuildInputs = [ pkgs.pkg-config ];
  buildInputs = [ pkgs.libvirt ];

  env.CGO_ENABLED = "1";

  postInstall = ''
    if [ -e "$out/bin/kvm" ]; then mv "$out/bin/kvm" "$out/bin/docker-machine-driver-kvm2"; fi
  '';

  meta.description = "minikube kvm2 driver, built from minikube source with synthesised cmd main";
}

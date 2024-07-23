# Red Hat Linux Binaries

This directory contains the Dockerfiles for building Windmill binaries for Red Hat Linux 9.

We build Windmill on the Red Hat Universal Base Image 9. Windmill requires the xmlsec1-devel package which is not available in the default UBI9 repositories. It is however included in the CodeReady Builder for RHEL9 repository which requires a RedHat subscription.
Moreover, only rust v1.75 is supported on Red Hat Linux 9. To make Windmill compatible with Rust v1.75, you need to pin the following libraries:
```
aws-config = "=1.4.0"
aws-sdk-sts = "=1.25.0"
aws-sdk-ssooidc = "=1.25.0"
aws-sdk-sso = "=1.25.0"
```

Make sure to include `aws-sdk-ssooidc` and `aws-sdk-sso` in the Cargo.toml of windmill-common as well to enforce the correct versions of the nested dependencies. Make them optional and include them in the `parquet` feature.
It's also possible that you need to add `#[async_recursion]` to the `lock_modules` function in the `backend/windmill-worker/src/worker_lockfiles.rs` file for it to compile.

Once the image is built, you can simply copy the binary on any Red Hat Linux 9 machine and run it. You will just need to install the xmlsec1 package which can be installed directly using `yum/dnf install xmlsec1`.



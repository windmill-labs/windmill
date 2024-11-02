# Red Hat Linux Binaries

This directory contains the Dockerfiles for building Windmill binaries for Red Hat Linux 9.

We build Windmill on the Red Hat Universal Base Image 9. Windmill requires the xmlsec1-devel package which is not available in the default UBI9 repositories. It is however included in the CodeReady Builder for RHEL9 repository which requires a RedHat subscription.

Once the image is built, you can simply copy the binary on any Red Hat Linux 9 machine and run it. You will just need to install the xmlsec1 package which can be installed directly using `yum/dnf install xmlsec1`.

## Notes
 - you will need to register on Red Hat and have an individual developer subscription and pass the username and password to docker build:
    ```
    docker build \
        -f docker/RHEL9/Dockerfile \
        --build-arg features="$features" \
        --secret id=rh_username,src=/path/to/rh_username \
        --secret id=rh_password,src=/path/to/rh_password \
        .
    ```
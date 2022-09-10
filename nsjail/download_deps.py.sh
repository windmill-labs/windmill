#/bin/sh

mkdir -p /tmp/dependencies
touch /tmp/dependencies/_windmill
/usr/local/bin/python3 -m pip install --cache-dir /tmp/.cache/pip -t /tmp/dependencies -r /user/requirements.txt\
    --no-color --isolated --no-warn-conflicts --disable-pip-version-check

mv /tmp/dependencies/* /out

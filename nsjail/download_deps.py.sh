#/bin/sh

INDEX_URL_ARG=$([ -z "$INDEX_URL" ] && echo ""|| echo "-i $INDEX_URL" )
EXTRA_INDEX_URL_ARG=$([ -z "$EXTRA_INDEX_URL" ] && echo ""|| echo "-i $EXTRA_INDEX_URL" )
TRUSTED_HOST_ARG=$([ -z "$TRUSTED_HOST" ] &&  echo "" || echo "--trusted-host $TRUSTED_HOST")

mkdir -p /tmp/dependencies
touch /tmp/dependencies/_windmill
/usr/local/bin/python3 -m pip install --cache-dir /tmp/.cache/pip -t /tmp/dependencies -r /user/requirements.txt\
    --no-color --isolated --no-warn-conflicts --disable-pip-version-check $INDEX_URL $EXTRA_INDEX_URL_ARG $TRUSTED_HOST_ARG

mv /tmp/dependencies/* /out

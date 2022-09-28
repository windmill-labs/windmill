#/bin/sh

INDEX_URL_ARG=$([ -z "$INDEX_URL" ] && echo ""|| echo "--index-url $INDEX_URL" )
EXTRA_INDEX_URL_ARG=$([ -z "$EXTRA_INDEX_URL" ] && echo ""|| echo "--extra-index-url $EXTRA_INDEX_URL" )
TRUSTED_HOST_ARG=$([ -z "$TRUSTED_HOST" ] &&  echo "" || echo "--trusted-host $TRUSTED_HOST")

if [ ! -z "$INDEX_URL" ]
then
      echo "\$INDEX_URL is set to $INDEX_URL"
fi

if [ ! -z "$EXTRA_INDEX_URL" ]
then
      echo "\$EXTRA_INDEX_URL is set to $EXTRA_INDEX_URL"
fi

if [ ! -z "$TRUSTED_HOST" ]
then
      echo "\$TRUSTED_HOST is set to $TRUSTED_HOST"
fi

mkdir -p /tmp/dependencies
touch /tmp/dependencies/_windmill
/usr/local/bin/python3 -m pip install --cache-dir /tmp/.cache/pip -t /tmp/dependencies -r /user/requirements.txt\
    --no-color --isolated --no-warn-conflicts --disable-pip-version-check $INDEX_URL_ARG $EXTRA_INDEX_URL_ARG $TRUSTED_HOST_ARG

mv /tmp/dependencies/* /out

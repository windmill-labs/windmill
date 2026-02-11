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

CMD="/usr/local/bin/uv pip install
\"$REQ\"
--target \"$TARGET\"
--no-cache
--no-config
--no-color
--no-deps
--link-mode=copy
$PY_PATH
$INDEX_URL_ARG $EXTRA_INDEX_URL_ARG $TRUSTED_HOST_ARG
--system
--reinstall
"

echo $CMD
eval $CMD

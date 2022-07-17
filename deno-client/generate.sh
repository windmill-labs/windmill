#!/bin/bash
set -e

/usr/local/bin/docker-entrypoint.sh generate -i ../backend/openapi.yaml -g typescript --additional-properties platform=deno -o windmill-api
sed -i 's/this\.type = "Job";//' windmill-api/models/Job.ts
sed -i '460 i \        if (mediaType === "text/plain") { return data }' windmill-api/models/ObjectSerializer.ts

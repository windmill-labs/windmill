#!/bin/bash
set -e

/usr/local/bin/docker-entrypoint.sh generate -i ../backend/openapi.yaml -g typescript --additional-properties platform=deno -o windmill-api
sed -i 's/this\.type = "Job";//' windmill-api/models/Job.ts
sed -i -z 's/public static parse(rawData: string, mediaType: string | undefined) {/public static parse(rawData: string, mediaType: string | undefined) {\n        if (mediaType === "text\/plain") { return rawData }/' windmill-api/models/ObjectSerializer.ts

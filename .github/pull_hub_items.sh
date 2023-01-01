#!/bin/bash

RT=$(curl -s https://hub.windmill.dev/resource_types/list | jq -c -r '.[]')
for item in ${RT[@]}; do
    name=$(jq -r '.name' <<< "$item")
    id=$(jq -r '.id' <<< "$item")
    echo $name $id 
    body=$(curl -s -H "accept: application/json" https://hub.windmill.dev/resource_types/${id}/${name})
    jq -r '.resource_type.schema' <<< "$body" > ./tmp
    description=$(jq -r '.resource_type.description' <<< "$body")
    description=$(echo -E $description)
    echo "{\"workspace_id\": \"admins\", \"name\": \"$name\", \"schema\": $(cat ./tmp), \"description\": \"$description\"} " | jq . > community/resource_types/${name}.json
    rm ./tmp
done

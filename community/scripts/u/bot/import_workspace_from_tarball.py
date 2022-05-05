import tarfile
import io
import wmill
import json

from windmill_api.api.script import get_script_by_path, create_script
from windmill_api.models.create_script_json_body import CreateScriptJsonBody
from windmill_api.models.create_script_json_body_schema import (
    CreateScriptJsonBodySchema,
)
from windmill_api.models.update_resource_json_body import UpdateResourceJsonBody
from windmill_api.models.create_resource_json_body import CreateResourceJsonBody

from windmill_api.models.update_resource_type_json_body import (
    UpdateResourceTypeJsonBody,
)

from windmill_api.models.create_resource_type_json_body import (
    CreateResourceTypeJsonBody,
)
from windmill_api.api.resource import (
    create_resource,
    create_resource_type,
    update_resource,
    update_resource_type,
    get_resource,
    get_resource_type,
)


client = wmill.Client()

SCRIPTS_PREFIX = "scripts/"
RESOURCES_PREFIX = "resources/"
RESOURCE_TYPES_PREFIX = "resource_types/"


def main(tarball: bytes, dry_run: bool = False):
    print("Tarball of size: {} bytes".format(len(tarball)))
    io_bytes = io.BytesIO(tarball)
    tar = tarfile.open(fileobj=io_bytes, mode="r")
    names = tar.getnames()

    for m in tar.getmembers():
        if m.name.startswith(SCRIPTS_PREFIX) and m.name.endswith(".py"):
            path = m.name[len(SCRIPTS_PREFIX) : -len(".py")]
            print("Processing script {}, {}".format(path, m.name))
            get_script_response = get_script_by_path.sync_detailed(
                workspace=client.workspace, path=path, client=client.client
            )

            json_tar_path = "{}.json".format(m.name[: -len(".py")])
            has_json = json_tar_path in names

            parent_hash = None
            summary = None
            description = None
            is_template = None
            schema = None

            content = tar.extractfile(m).read().decode("utf-8")

            if get_script_response.status_code == 200:
                old_script = json.loads(get_script_response.content)

                if old_script["content"] == content:
                    if not has_json:
                        print(
                            "same content and no metadata for this script in tarball, no need to update"
                        )
                        continue

                    json_content = tar.extractfile(tar.getmember(json_tar_path)).read()
                    metadata = json.loads(json_content)

                    if (
                        old_script.get("summary") == metadata.get("summary")
                        and old_script.get("description") == metadata.get("description")
                        and old_script.get("is_template") == metadata.get("is_template")
                        and old_script.get("schema") == metadata.get("schema")
                        and old_script.get("lock") == metadata.get("lock")
                    ):
                        print("same content and no metadata, no need to update")
                        continue

                parent_hash = old_script["hash"]
                summary = old_script.get("summary")
                description = old_script.get("description")
                is_template = old_script.get("is_template")
                schema = old_script.get("schema")
                lock = None

            if has_json:
                json_content = tar.extractfile(tar.getmember(json_tar_path)).read()
                metadata = json.loads(json_content)

                summary = metadata.get("summary")
                description = metadata.get("description")
                is_template = metadata.get("is_template")
                schema = metadata.get("schema")
                lock = metadata.get("lock")
                if lock == []:
                    lock = None

            if schema:
                schema = CreateScriptJsonBodySchema.from_dict(schema)

            print("Uploading new version of script at path: {}".format(path))

            if dry_run:
                print("Skipped because dry-run")
            else:
                r = create_script.sync_detailed(
                    client.workspace,
                    client=client.client,
                    json_body=CreateScriptJsonBody(
                        content=content,
                        path=path,
                        parent_hash=parent_hash,
                        summary=summary,
                        description=description,
                        is_template=is_template,
                        schema=schema,
                        lock=lock,
                    ),
                )
                print(r.content)
        if m.name.startswith(RESOURCES_PREFIX) and m.name.endswith(".json"):
            path = m.name[len(RESOURCES_PREFIX) : -len(".json")]
            print("Processing resource {}, {}".format(path, m.name))
            get_resource_response = get_resource.sync_detailed(
                workspace=client.workspace, path=path, client=client.client
            )
            content = tar.extractfile(m).read().decode("utf-8")
            resource = json.loads(content)
            if get_resource_response.status_code == 200:
                old_resource = json.loads(get_resource_response.content)
                if resource["value"] != old_resource["value"]:
                    print("Updating existing resource")
                    r = update_resource.sync_detailed(
                        client.workspace,
                        path=path,
                        json_body=UpdateResourceJsonBody.from_dict(resource),
                        client=client.client,
                    )
                    print(r)
                else:
                    print("Skipping updating identical resource")
            else:
                print("Creating new resource")
                r = create_resource.sync_detailed(
                    client.workspace,
                    json_body=CreateResourceJsonBody.from_dict(resource),
                    client=client.client,
                )
                print(r)

        if m.name.startswith(RESOURCE_TYPES_PREFIX) and m.name.endswith(".json"):
            path = m.name[len(RESOURCE_TYPES_PREFIX) : -len(".json")]
            print("Processing resource type {}, {}".format(path, m.name))
            get_resource_response = get_resource_type.sync_detailed(
                workspace=client.workspace, path=path, client=client.client
            )
            content = tar.extractfile(m).read().decode("utf-8")
            resource = json.loads(content)
            print(resource)
            if get_resource_response.status_code == 200:
                old_resource_type = json.loads(get_resource_response.content)
                if resource["schema"] != old_resource_type["schema"]:
                    print("Updating existing resource type")
                    r = update_resource_type.sync_detailed(
                        client.workspace,
                        path=path,
                        json_body=UpdateResourceTypeJsonBody.from_dict(resource),
                        client=client.client,
                    )
                    print(r)
                else:
                    print("Skipping updating identical resource type")
            else:
                print("Creating new resource type")
                r = create_resource_type.sync_detailed(
                    client.workspace,
                    json_body=CreateResourceTypeJsonBody.from_dict(resource),
                    client=client.client,
                )
                print(r)

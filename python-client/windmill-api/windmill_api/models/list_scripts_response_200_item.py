import datetime
from typing import Any, Dict, List, Type, TypeVar, Union, cast

import attr
from dateutil.parser import isoparse

from ..models.list_scripts_response_200_item_extra_perms import ListScriptsResponse200ItemExtraPerms
from ..models.list_scripts_response_200_item_kind import ListScriptsResponse200ItemKind
from ..models.list_scripts_response_200_item_language import ListScriptsResponse200ItemLanguage
from ..models.list_scripts_response_200_item_schema import ListScriptsResponse200ItemSchema
from ..types import UNSET, Unset

T = TypeVar("T", bound="ListScriptsResponse200Item")


@attr.s(auto_attribs=True)
class ListScriptsResponse200Item:
    """
    Attributes:
        hash_ (str):
        path (str):
        summary (str):
        content (str):
        created_by (str):
        created_at (datetime.datetime):
        archived (bool):
        deleted (bool):
        is_template (bool):
        extra_perms (ListScriptsResponse200ItemExtraPerms):
        language (ListScriptsResponse200ItemLanguage):
        kind (ListScriptsResponse200ItemKind):
        workspace_id (Union[Unset, str]):
        parent_hashes (Union[Unset, List[str]]): The first element is the direct parent of the script, the second is the
            parent of the first, etc
        description (Union[Unset, str]):
        schema (Union[Unset, ListScriptsResponse200ItemSchema]):
        lock (Union[Unset, str]):
        lock_error_logs (Union[Unset, str]):
    """

    hash_: str
    path: str
    summary: str
    content: str
    created_by: str
    created_at: datetime.datetime
    archived: bool
    deleted: bool
    is_template: bool
    extra_perms: ListScriptsResponse200ItemExtraPerms
    language: ListScriptsResponse200ItemLanguage
    kind: ListScriptsResponse200ItemKind
    workspace_id: Union[Unset, str] = UNSET
    parent_hashes: Union[Unset, List[str]] = UNSET
    description: Union[Unset, str] = UNSET
    schema: Union[Unset, ListScriptsResponse200ItemSchema] = UNSET
    lock: Union[Unset, str] = UNSET
    lock_error_logs: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        hash_ = self.hash_
        path = self.path
        summary = self.summary
        content = self.content
        created_by = self.created_by
        created_at = self.created_at.isoformat()

        archived = self.archived
        deleted = self.deleted
        is_template = self.is_template
        extra_perms = self.extra_perms.to_dict()

        language = self.language.value

        kind = self.kind.value

        workspace_id = self.workspace_id
        parent_hashes: Union[Unset, List[str]] = UNSET
        if not isinstance(self.parent_hashes, Unset):
            parent_hashes = self.parent_hashes

        description = self.description
        schema: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.schema, Unset):
            schema = self.schema.to_dict()

        lock = self.lock
        lock_error_logs = self.lock_error_logs

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "hash": hash_,
                "path": path,
                "summary": summary,
                "content": content,
                "created_by": created_by,
                "created_at": created_at,
                "archived": archived,
                "deleted": deleted,
                "is_template": is_template,
                "extra_perms": extra_perms,
                "language": language,
                "kind": kind,
            }
        )
        if workspace_id is not UNSET:
            field_dict["workspace_id"] = workspace_id
        if parent_hashes is not UNSET:
            field_dict["parent_hashes"] = parent_hashes
        if description is not UNSET:
            field_dict["description"] = description
        if schema is not UNSET:
            field_dict["schema"] = schema
        if lock is not UNSET:
            field_dict["lock"] = lock
        if lock_error_logs is not UNSET:
            field_dict["lock_error_logs"] = lock_error_logs

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        hash_ = d.pop("hash")

        path = d.pop("path")

        summary = d.pop("summary")

        content = d.pop("content")

        created_by = d.pop("created_by")

        created_at = isoparse(d.pop("created_at"))

        archived = d.pop("archived")

        deleted = d.pop("deleted")

        is_template = d.pop("is_template")

        extra_perms = ListScriptsResponse200ItemExtraPerms.from_dict(d.pop("extra_perms"))

        language = ListScriptsResponse200ItemLanguage(d.pop("language"))

        kind = ListScriptsResponse200ItemKind(d.pop("kind"))

        workspace_id = d.pop("workspace_id", UNSET)

        parent_hashes = cast(List[str], d.pop("parent_hashes", UNSET))

        description = d.pop("description", UNSET)

        _schema = d.pop("schema", UNSET)
        schema: Union[Unset, ListScriptsResponse200ItemSchema]
        if isinstance(_schema, Unset):
            schema = UNSET
        else:
            schema = ListScriptsResponse200ItemSchema.from_dict(_schema)

        lock = d.pop("lock", UNSET)

        lock_error_logs = d.pop("lock_error_logs", UNSET)

        list_scripts_response_200_item = cls(
            hash_=hash_,
            path=path,
            summary=summary,
            content=content,
            created_by=created_by,
            created_at=created_at,
            archived=archived,
            deleted=deleted,
            is_template=is_template,
            extra_perms=extra_perms,
            language=language,
            kind=kind,
            workspace_id=workspace_id,
            parent_hashes=parent_hashes,
            description=description,
            schema=schema,
            lock=lock,
            lock_error_logs=lock_error_logs,
        )

        list_scripts_response_200_item.additional_properties = d
        return list_scripts_response_200_item

    @property
    def additional_keys(self) -> List[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> Any:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties

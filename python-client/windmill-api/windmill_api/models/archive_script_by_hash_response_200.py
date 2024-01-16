from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from typing import cast, List
from typing import Dict
from ..models.archive_script_by_hash_response_200_kind import ArchiveScriptByHashResponse200Kind
import datetime
from dateutil.parser import isoparse
from ..types import UNSET, Unset
from ..models.archive_script_by_hash_response_200_language import ArchiveScriptByHashResponse200Language

if TYPE_CHECKING:
  from ..models.archive_script_by_hash_response_200_extra_perms import ArchiveScriptByHashResponse200ExtraPerms
  from ..models.archive_script_by_hash_response_200_schema import ArchiveScriptByHashResponse200Schema





T = TypeVar("T", bound="ArchiveScriptByHashResponse200")


@_attrs_define
class ArchiveScriptByHashResponse200:
    """ 
        Attributes:
            hash_ (str):
            path (str):
            summary (str):
            description (str):
            content (str):
            created_by (str):
            created_at (datetime.datetime):
            archived (bool):
            deleted (bool):
            is_template (bool):
            extra_perms (ArchiveScriptByHashResponse200ExtraPerms):
            language (ArchiveScriptByHashResponse200Language):
            kind (ArchiveScriptByHashResponse200Kind):
            starred (bool):
            workspace_id (Union[Unset, str]):
            parent_hashes (Union[Unset, List[str]]): The first element is the direct parent of the script, the second is the
                parent of the first, etc
            schema (Union[Unset, ArchiveScriptByHashResponse200Schema]):
            lock (Union[Unset, str]):
            lock_error_logs (Union[Unset, str]):
            tag (Union[Unset, str]):
            has_draft (Union[Unset, bool]):
            draft_only (Union[Unset, bool]):
            envs (Union[Unset, List[str]]):
            concurrent_limit (Union[Unset, int]):
            concurrency_time_window_s (Union[Unset, int]):
            cache_ttl (Union[Unset, float]):
            dedicated_worker (Union[Unset, bool]):
            ws_error_handler_muted (Union[Unset, bool]):
            priority (Union[Unset, int]):
            restart_unless_cancelled (Union[Unset, bool]):
            timeout (Union[Unset, int]):
            delete_after_use (Union[Unset, bool]):
     """

    hash_: str
    path: str
    summary: str
    description: str
    content: str
    created_by: str
    created_at: datetime.datetime
    archived: bool
    deleted: bool
    is_template: bool
    extra_perms: 'ArchiveScriptByHashResponse200ExtraPerms'
    language: ArchiveScriptByHashResponse200Language
    kind: ArchiveScriptByHashResponse200Kind
    starred: bool
    workspace_id: Union[Unset, str] = UNSET
    parent_hashes: Union[Unset, List[str]] = UNSET
    schema: Union[Unset, 'ArchiveScriptByHashResponse200Schema'] = UNSET
    lock: Union[Unset, str] = UNSET
    lock_error_logs: Union[Unset, str] = UNSET
    tag: Union[Unset, str] = UNSET
    has_draft: Union[Unset, bool] = UNSET
    draft_only: Union[Unset, bool] = UNSET
    envs: Union[Unset, List[str]] = UNSET
    concurrent_limit: Union[Unset, int] = UNSET
    concurrency_time_window_s: Union[Unset, int] = UNSET
    cache_ttl: Union[Unset, float] = UNSET
    dedicated_worker: Union[Unset, bool] = UNSET
    ws_error_handler_muted: Union[Unset, bool] = UNSET
    priority: Union[Unset, int] = UNSET
    restart_unless_cancelled: Union[Unset, bool] = UNSET
    timeout: Union[Unset, int] = UNSET
    delete_after_use: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.archive_script_by_hash_response_200_extra_perms import ArchiveScriptByHashResponse200ExtraPerms
        from ..models.archive_script_by_hash_response_200_schema import ArchiveScriptByHashResponse200Schema
        hash_ = self.hash_
        path = self.path
        summary = self.summary
        description = self.description
        content = self.content
        created_by = self.created_by
        created_at = self.created_at.isoformat()

        archived = self.archived
        deleted = self.deleted
        is_template = self.is_template
        extra_perms = self.extra_perms.to_dict()

        language = self.language.value

        kind = self.kind.value

        starred = self.starred
        workspace_id = self.workspace_id
        parent_hashes: Union[Unset, List[str]] = UNSET
        if not isinstance(self.parent_hashes, Unset):
            parent_hashes = self.parent_hashes




        schema: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.schema, Unset):
            schema = self.schema.to_dict()

        lock = self.lock
        lock_error_logs = self.lock_error_logs
        tag = self.tag
        has_draft = self.has_draft
        draft_only = self.draft_only
        envs: Union[Unset, List[str]] = UNSET
        if not isinstance(self.envs, Unset):
            envs = self.envs




        concurrent_limit = self.concurrent_limit
        concurrency_time_window_s = self.concurrency_time_window_s
        cache_ttl = self.cache_ttl
        dedicated_worker = self.dedicated_worker
        ws_error_handler_muted = self.ws_error_handler_muted
        priority = self.priority
        restart_unless_cancelled = self.restart_unless_cancelled
        timeout = self.timeout
        delete_after_use = self.delete_after_use

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "hash": hash_,
            "path": path,
            "summary": summary,
            "description": description,
            "content": content,
            "created_by": created_by,
            "created_at": created_at,
            "archived": archived,
            "deleted": deleted,
            "is_template": is_template,
            "extra_perms": extra_perms,
            "language": language,
            "kind": kind,
            "starred": starred,
        })
        if workspace_id is not UNSET:
            field_dict["workspace_id"] = workspace_id
        if parent_hashes is not UNSET:
            field_dict["parent_hashes"] = parent_hashes
        if schema is not UNSET:
            field_dict["schema"] = schema
        if lock is not UNSET:
            field_dict["lock"] = lock
        if lock_error_logs is not UNSET:
            field_dict["lock_error_logs"] = lock_error_logs
        if tag is not UNSET:
            field_dict["tag"] = tag
        if has_draft is not UNSET:
            field_dict["has_draft"] = has_draft
        if draft_only is not UNSET:
            field_dict["draft_only"] = draft_only
        if envs is not UNSET:
            field_dict["envs"] = envs
        if concurrent_limit is not UNSET:
            field_dict["concurrent_limit"] = concurrent_limit
        if concurrency_time_window_s is not UNSET:
            field_dict["concurrency_time_window_s"] = concurrency_time_window_s
        if cache_ttl is not UNSET:
            field_dict["cache_ttl"] = cache_ttl
        if dedicated_worker is not UNSET:
            field_dict["dedicated_worker"] = dedicated_worker
        if ws_error_handler_muted is not UNSET:
            field_dict["ws_error_handler_muted"] = ws_error_handler_muted
        if priority is not UNSET:
            field_dict["priority"] = priority
        if restart_unless_cancelled is not UNSET:
            field_dict["restart_unless_cancelled"] = restart_unless_cancelled
        if timeout is not UNSET:
            field_dict["timeout"] = timeout
        if delete_after_use is not UNSET:
            field_dict["delete_after_use"] = delete_after_use

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.archive_script_by_hash_response_200_extra_perms import ArchiveScriptByHashResponse200ExtraPerms
        from ..models.archive_script_by_hash_response_200_schema import ArchiveScriptByHashResponse200Schema
        d = src_dict.copy()
        hash_ = d.pop("hash")

        path = d.pop("path")

        summary = d.pop("summary")

        description = d.pop("description")

        content = d.pop("content")

        created_by = d.pop("created_by")

        created_at = isoparse(d.pop("created_at"))




        archived = d.pop("archived")

        deleted = d.pop("deleted")

        is_template = d.pop("is_template")

        extra_perms = ArchiveScriptByHashResponse200ExtraPerms.from_dict(d.pop("extra_perms"))




        language = ArchiveScriptByHashResponse200Language(d.pop("language"))




        kind = ArchiveScriptByHashResponse200Kind(d.pop("kind"))




        starred = d.pop("starred")

        workspace_id = d.pop("workspace_id", UNSET)

        parent_hashes = cast(List[str], d.pop("parent_hashes", UNSET))


        _schema = d.pop("schema", UNSET)
        schema: Union[Unset, ArchiveScriptByHashResponse200Schema]
        if isinstance(_schema,  Unset):
            schema = UNSET
        else:
            schema = ArchiveScriptByHashResponse200Schema.from_dict(_schema)




        lock = d.pop("lock", UNSET)

        lock_error_logs = d.pop("lock_error_logs", UNSET)

        tag = d.pop("tag", UNSET)

        has_draft = d.pop("has_draft", UNSET)

        draft_only = d.pop("draft_only", UNSET)

        envs = cast(List[str], d.pop("envs", UNSET))


        concurrent_limit = d.pop("concurrent_limit", UNSET)

        concurrency_time_window_s = d.pop("concurrency_time_window_s", UNSET)

        cache_ttl = d.pop("cache_ttl", UNSET)

        dedicated_worker = d.pop("dedicated_worker", UNSET)

        ws_error_handler_muted = d.pop("ws_error_handler_muted", UNSET)

        priority = d.pop("priority", UNSET)

        restart_unless_cancelled = d.pop("restart_unless_cancelled", UNSET)

        timeout = d.pop("timeout", UNSET)

        delete_after_use = d.pop("delete_after_use", UNSET)

        archive_script_by_hash_response_200 = cls(
            hash_=hash_,
            path=path,
            summary=summary,
            description=description,
            content=content,
            created_by=created_by,
            created_at=created_at,
            archived=archived,
            deleted=deleted,
            is_template=is_template,
            extra_perms=extra_perms,
            language=language,
            kind=kind,
            starred=starred,
            workspace_id=workspace_id,
            parent_hashes=parent_hashes,
            schema=schema,
            lock=lock,
            lock_error_logs=lock_error_logs,
            tag=tag,
            has_draft=has_draft,
            draft_only=draft_only,
            envs=envs,
            concurrent_limit=concurrent_limit,
            concurrency_time_window_s=concurrency_time_window_s,
            cache_ttl=cache_ttl,
            dedicated_worker=dedicated_worker,
            ws_error_handler_muted=ws_error_handler_muted,
            priority=priority,
            restart_unless_cancelled=restart_unless_cancelled,
            timeout=timeout,
            delete_after_use=delete_after_use,
        )

        archive_script_by_hash_response_200.additional_properties = d
        return archive_script_by_hash_response_200

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

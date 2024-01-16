from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from typing import Dict
import datetime
from dateutil.parser import isoparse
from ..types import UNSET, Unset

if TYPE_CHECKING:
  from ..models.flow_metadata_extra_perms import FlowMetadataExtraPerms





T = TypeVar("T", bound="FlowMetadata")


@_attrs_define
class FlowMetadata:
    """ 
        Attributes:
            path (str):
            edited_by (str):
            edited_at (datetime.datetime):
            archived (bool):
            extra_perms (FlowMetadataExtraPerms):
            workspace_id (Union[Unset, str]):
            additional_properties (Union[Unset, bool]):
            starred (Union[Unset, bool]):
            draft_only (Union[Unset, bool]):
            tag (Union[Unset, str]):
            ws_error_handler_muted (Union[Unset, bool]):
            priority (Union[Unset, int]):
            dedicated_worker (Union[Unset, bool]):
            timeout (Union[Unset, float]):
     """

    path: str
    edited_by: str
    edited_at: datetime.datetime
    archived: bool
    extra_perms: 'FlowMetadataExtraPerms'
    workspace_id: Union[Unset, str] = UNSET
    additional_properties: Union[Unset, bool] = UNSET
    starred: Union[Unset, bool] = UNSET
    draft_only: Union[Unset, bool] = UNSET
    tag: Union[Unset, str] = UNSET
    ws_error_handler_muted: Union[Unset, bool] = UNSET
    priority: Union[Unset, int] = UNSET
    dedicated_worker: Union[Unset, bool] = UNSET
    timeout: Union[Unset, float] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.flow_metadata_extra_perms import FlowMetadataExtraPerms
        path = self.path
        edited_by = self.edited_by
        edited_at = self.edited_at.isoformat()

        archived = self.archived
        extra_perms = self.extra_perms.to_dict()

        workspace_id = self.workspace_id
        additional_properties = self.additional_properties
        starred = self.starred
        draft_only = self.draft_only
        tag = self.tag
        ws_error_handler_muted = self.ws_error_handler_muted
        priority = self.priority
        dedicated_worker = self.dedicated_worker
        timeout = self.timeout

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "path": path,
            "edited_by": edited_by,
            "edited_at": edited_at,
            "archived": archived,
            "extra_perms": extra_perms,
        })
        if workspace_id is not UNSET:
            field_dict["workspace_id"] = workspace_id
        if additional_properties is not UNSET:
            field_dict["additionalProperties"] = additional_properties
        if starred is not UNSET:
            field_dict["starred"] = starred
        if draft_only is not UNSET:
            field_dict["draft_only"] = draft_only
        if tag is not UNSET:
            field_dict["tag"] = tag
        if ws_error_handler_muted is not UNSET:
            field_dict["ws_error_handler_muted"] = ws_error_handler_muted
        if priority is not UNSET:
            field_dict["priority"] = priority
        if dedicated_worker is not UNSET:
            field_dict["dedicated_worker"] = dedicated_worker
        if timeout is not UNSET:
            field_dict["timeout"] = timeout

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.flow_metadata_extra_perms import FlowMetadataExtraPerms
        d = src_dict.copy()
        path = d.pop("path")

        edited_by = d.pop("edited_by")

        edited_at = isoparse(d.pop("edited_at"))




        archived = d.pop("archived")

        extra_perms = FlowMetadataExtraPerms.from_dict(d.pop("extra_perms"))




        workspace_id = d.pop("workspace_id", UNSET)

        additional_properties = d.pop("additionalProperties", UNSET)

        starred = d.pop("starred", UNSET)

        draft_only = d.pop("draft_only", UNSET)

        tag = d.pop("tag", UNSET)

        ws_error_handler_muted = d.pop("ws_error_handler_muted", UNSET)

        priority = d.pop("priority", UNSET)

        dedicated_worker = d.pop("dedicated_worker", UNSET)

        timeout = d.pop("timeout", UNSET)

        flow_metadata = cls(
            path=path,
            edited_by=edited_by,
            edited_at=edited_at,
            archived=archived,
            extra_perms=extra_perms,
            workspace_id=workspace_id,
            additional_properties=additional_properties,
            starred=starred,
            draft_only=draft_only,
            tag=tag,
            ws_error_handler_muted=ws_error_handler_muted,
            priority=priority,
            dedicated_worker=dedicated_worker,
            timeout=timeout,
        )

        flow_metadata.additional_properties = d
        return flow_metadata

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

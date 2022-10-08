import datetime
from typing import Any, Dict, List, Type, TypeVar, Union

import attr
from dateutil.parser import isoparse

from ..models.flow_metadata_extra_perms import FlowMetadataExtraPerms
from ..types import UNSET, Unset

T = TypeVar("T", bound="FlowMetadata")


@attr.s(auto_attribs=True)
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
    """

    path: str
    edited_by: str
    edited_at: datetime.datetime
    archived: bool
    extra_perms: FlowMetadataExtraPerms
    workspace_id: Union[Unset, str] = UNSET
    additional_properties: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        path = self.path
        edited_by = self.edited_by
        edited_at = self.edited_at.isoformat()

        archived = self.archived
        extra_perms = self.extra_perms.to_dict()

        workspace_id = self.workspace_id
        additional_properties = self.additional_properties

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "path": path,
                "edited_by": edited_by,
                "edited_at": edited_at,
                "archived": archived,
                "extra_perms": extra_perms,
            }
        )
        if workspace_id is not UNSET:
            field_dict["workspace_id"] = workspace_id
        if additional_properties is not UNSET:
            field_dict["additionalProperties"] = additional_properties

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        path = d.pop("path")

        edited_by = d.pop("edited_by")

        edited_at = isoparse(d.pop("edited_at"))

        archived = d.pop("archived")

        extra_perms = FlowMetadataExtraPerms.from_dict(d.pop("extra_perms"))

        workspace_id = d.pop("workspace_id", UNSET)

        additional_properties = d.pop("additionalProperties", UNSET)

        flow_metadata = cls(
            path=path,
            edited_by=edited_by,
            edited_at=edited_at,
            archived=archived,
            extra_perms=extra_perms,
            workspace_id=workspace_id,
            additional_properties=additional_properties,
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

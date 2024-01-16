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
  from ..models.listable_raw_app_extra_perms import ListableRawAppExtraPerms





T = TypeVar("T", bound="ListableRawApp")


@_attrs_define
class ListableRawApp:
    """ 
        Attributes:
            workspace_id (str):
            path (str):
            summary (str):
            extra_perms (ListableRawAppExtraPerms):
            version (float):
            edited_at (datetime.datetime):
            starred (Union[Unset, bool]):
     """

    workspace_id: str
    path: str
    summary: str
    extra_perms: 'ListableRawAppExtraPerms'
    version: float
    edited_at: datetime.datetime
    starred: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.listable_raw_app_extra_perms import ListableRawAppExtraPerms
        workspace_id = self.workspace_id
        path = self.path
        summary = self.summary
        extra_perms = self.extra_perms.to_dict()

        version = self.version
        edited_at = self.edited_at.isoformat()

        starred = self.starred

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "workspace_id": workspace_id,
            "path": path,
            "summary": summary,
            "extra_perms": extra_perms,
            "version": version,
            "edited_at": edited_at,
        })
        if starred is not UNSET:
            field_dict["starred"] = starred

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.listable_raw_app_extra_perms import ListableRawAppExtraPerms
        d = src_dict.copy()
        workspace_id = d.pop("workspace_id")

        path = d.pop("path")

        summary = d.pop("summary")

        extra_perms = ListableRawAppExtraPerms.from_dict(d.pop("extra_perms"))




        version = d.pop("version")

        edited_at = isoparse(d.pop("edited_at"))




        starred = d.pop("starred", UNSET)

        listable_raw_app = cls(
            workspace_id=workspace_id,
            path=path,
            summary=summary,
            extra_perms=extra_perms,
            version=version,
            edited_at=edited_at,
            starred=starred,
        )

        listable_raw_app.additional_properties = d
        return listable_raw_app

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

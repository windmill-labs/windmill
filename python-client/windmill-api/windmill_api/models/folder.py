from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import cast
from typing import cast, List
from typing import Dict

if TYPE_CHECKING:
  from ..models.folder_extra_perms import FolderExtraPerms





T = TypeVar("T", bound="Folder")


@_attrs_define
class Folder:
    """ 
        Attributes:
            name (str):
            owners (List[str]):
            extra_perms (FolderExtraPerms):
     """

    name: str
    owners: List[str]
    extra_perms: 'FolderExtraPerms'
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.folder_extra_perms import FolderExtraPerms
        name = self.name
        owners = self.owners




        extra_perms = self.extra_perms.to_dict()


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "name": name,
            "owners": owners,
            "extra_perms": extra_perms,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.folder_extra_perms import FolderExtraPerms
        d = src_dict.copy()
        name = d.pop("name")

        owners = cast(List[str], d.pop("owners"))


        extra_perms = FolderExtraPerms.from_dict(d.pop("extra_perms"))




        folder = cls(
            name=name,
            owners=owners,
            extra_perms=extra_perms,
        )

        folder.additional_properties = d
        return folder

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

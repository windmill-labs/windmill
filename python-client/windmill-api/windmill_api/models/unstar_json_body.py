from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..models.unstar_json_body_favorite_kind import UnstarJsonBodyFavoriteKind
from ..types import UNSET, Unset






T = TypeVar("T", bound="UnstarJsonBody")


@_attrs_define
class UnstarJsonBody:
    """ 
        Attributes:
            path (Union[Unset, str]):
            favorite_kind (Union[Unset, UnstarJsonBodyFavoriteKind]):
     """

    path: Union[Unset, str] = UNSET
    favorite_kind: Union[Unset, UnstarJsonBodyFavoriteKind] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        path = self.path
        favorite_kind: Union[Unset, str] = UNSET
        if not isinstance(self.favorite_kind, Unset):
            favorite_kind = self.favorite_kind.value


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if path is not UNSET:
            field_dict["path"] = path
        if favorite_kind is not UNSET:
            field_dict["favorite_kind"] = favorite_kind

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        path = d.pop("path", UNSET)

        _favorite_kind = d.pop("favorite_kind", UNSET)
        favorite_kind: Union[Unset, UnstarJsonBodyFavoriteKind]
        if isinstance(_favorite_kind,  Unset):
            favorite_kind = UNSET
        else:
            favorite_kind = UnstarJsonBodyFavoriteKind(_favorite_kind)




        unstar_json_body = cls(
            path=path,
            favorite_kind=favorite_kind,
        )

        unstar_json_body.additional_properties = d
        return unstar_json_body

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

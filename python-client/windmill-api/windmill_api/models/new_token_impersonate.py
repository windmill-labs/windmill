from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
import datetime
from dateutil.parser import isoparse
from ..types import UNSET, Unset






T = TypeVar("T", bound="NewTokenImpersonate")


@_attrs_define
class NewTokenImpersonate:
    """ 
        Attributes:
            impersonate_email (str):
            label (Union[Unset, str]):
            expiration (Union[Unset, datetime.datetime]):
     """

    impersonate_email: str
    label: Union[Unset, str] = UNSET
    expiration: Union[Unset, datetime.datetime] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        impersonate_email = self.impersonate_email
        label = self.label
        expiration: Union[Unset, str] = UNSET
        if not isinstance(self.expiration, Unset):
            expiration = self.expiration.isoformat()


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "impersonate_email": impersonate_email,
        })
        if label is not UNSET:
            field_dict["label"] = label
        if expiration is not UNSET:
            field_dict["expiration"] = expiration

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        impersonate_email = d.pop("impersonate_email")

        label = d.pop("label", UNSET)

        _expiration = d.pop("expiration", UNSET)
        expiration: Union[Unset, datetime.datetime]
        if isinstance(_expiration,  Unset):
            expiration = UNSET
        else:
            expiration = isoparse(_expiration)




        new_token_impersonate = cls(
            impersonate_email=impersonate_email,
            label=label,
            expiration=expiration,
        )

        new_token_impersonate.additional_properties = d
        return new_token_impersonate

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

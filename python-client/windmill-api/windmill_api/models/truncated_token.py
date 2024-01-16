from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from typing import cast, List
import datetime
from dateutil.parser import isoparse
from ..types import UNSET, Unset






T = TypeVar("T", bound="TruncatedToken")


@_attrs_define
class TruncatedToken:
    """ 
        Attributes:
            token_prefix (str):
            created_at (datetime.datetime):
            last_used_at (datetime.datetime):
            label (Union[Unset, str]):
            expiration (Union[Unset, datetime.datetime]):
            scopes (Union[Unset, List[str]]):
     """

    token_prefix: str
    created_at: datetime.datetime
    last_used_at: datetime.datetime
    label: Union[Unset, str] = UNSET
    expiration: Union[Unset, datetime.datetime] = UNSET
    scopes: Union[Unset, List[str]] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        token_prefix = self.token_prefix
        created_at = self.created_at.isoformat()

        last_used_at = self.last_used_at.isoformat()

        label = self.label
        expiration: Union[Unset, str] = UNSET
        if not isinstance(self.expiration, Unset):
            expiration = self.expiration.isoformat()

        scopes: Union[Unset, List[str]] = UNSET
        if not isinstance(self.scopes, Unset):
            scopes = self.scopes





        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "token_prefix": token_prefix,
            "created_at": created_at,
            "last_used_at": last_used_at,
        })
        if label is not UNSET:
            field_dict["label"] = label
        if expiration is not UNSET:
            field_dict["expiration"] = expiration
        if scopes is not UNSET:
            field_dict["scopes"] = scopes

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        token_prefix = d.pop("token_prefix")

        created_at = isoparse(d.pop("created_at"))




        last_used_at = isoparse(d.pop("last_used_at"))




        label = d.pop("label", UNSET)

        _expiration = d.pop("expiration", UNSET)
        expiration: Union[Unset, datetime.datetime]
        if isinstance(_expiration,  Unset):
            expiration = UNSET
        else:
            expiration = isoparse(_expiration)




        scopes = cast(List[str], d.pop("scopes", UNSET))


        truncated_token = cls(
            token_prefix=token_prefix,
            created_at=created_at,
            last_used_at=last_used_at,
            label=label,
            expiration=expiration,
            scopes=scopes,
        )

        truncated_token.additional_properties = d
        return truncated_token

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

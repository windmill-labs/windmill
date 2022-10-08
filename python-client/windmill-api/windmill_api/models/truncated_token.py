import datetime
from typing import Any, Dict, List, Type, TypeVar, Union

import attr
from dateutil.parser import isoparse

from ..types import UNSET, Unset

T = TypeVar("T", bound="TruncatedToken")


@attr.s(auto_attribs=True)
class TruncatedToken:
    """
    Attributes:
        token_prefix (str):
        created_at (datetime.datetime):
        last_used_at (datetime.datetime):
        label (Union[Unset, str]):
        expiration (Union[Unset, datetime.datetime]):
    """

    token_prefix: str
    created_at: datetime.datetime
    last_used_at: datetime.datetime
    label: Union[Unset, str] = UNSET
    expiration: Union[Unset, datetime.datetime] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        token_prefix = self.token_prefix
        created_at = self.created_at.isoformat()

        last_used_at = self.last_used_at.isoformat()

        label = self.label
        expiration: Union[Unset, str] = UNSET
        if not isinstance(self.expiration, Unset):
            expiration = self.expiration.isoformat()

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "token_prefix": token_prefix,
                "created_at": created_at,
                "last_used_at": last_used_at,
            }
        )
        if label is not UNSET:
            field_dict["label"] = label
        if expiration is not UNSET:
            field_dict["expiration"] = expiration

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
        if isinstance(_expiration, Unset):
            expiration = UNSET
        else:
            expiration = isoparse(_expiration)

        truncated_token = cls(
            token_prefix=token_prefix,
            created_at=created_at,
            last_used_at=last_used_at,
            label=label,
            expiration=expiration,
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

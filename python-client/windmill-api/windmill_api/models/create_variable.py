from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..types import UNSET, Unset

T = TypeVar("T", bound="CreateVariable")


@attr.s(auto_attribs=True)
class CreateVariable:
    """
    Attributes:
        path (str):
        value (str):
        is_secret (bool):
        description (str):
        account (Union[Unset, int]):
        is_oauth (Union[Unset, bool]):
    """

    path: str
    value: str
    is_secret: bool
    description: str
    account: Union[Unset, int] = UNSET
    is_oauth: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        path = self.path
        value = self.value
        is_secret = self.is_secret
        description = self.description
        account = self.account
        is_oauth = self.is_oauth

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "path": path,
                "value": value,
                "is_secret": is_secret,
                "description": description,
            }
        )
        if account is not UNSET:
            field_dict["account"] = account
        if is_oauth is not UNSET:
            field_dict["is_oauth"] = is_oauth

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        path = d.pop("path")

        value = d.pop("value")

        is_secret = d.pop("is_secret")

        description = d.pop("description")

        account = d.pop("account", UNSET)

        is_oauth = d.pop("is_oauth", UNSET)

        create_variable = cls(
            path=path,
            value=value,
            is_secret=is_secret,
            description=description,
            account=account,
            is_oauth=is_oauth,
        )

        create_variable.additional_properties = d
        return create_variable

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

from typing import Any, Dict, List, Type, TypeVar, Union, cast

import attr

from ..types import UNSET, Unset

T = TypeVar("T", bound="TokenResponse")


@attr.s(auto_attribs=True)
class TokenResponse:
    """
    Attributes:
        access_token (str):
        expires_in (Union[Unset, int]):
        refresh_token (Union[Unset, str]):
        scope (Union[Unset, List[str]]):
    """

    access_token: str
    expires_in: Union[Unset, int] = UNSET
    refresh_token: Union[Unset, str] = UNSET
    scope: Union[Unset, List[str]] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        access_token = self.access_token
        expires_in = self.expires_in
        refresh_token = self.refresh_token
        scope: Union[Unset, List[str]] = UNSET
        if not isinstance(self.scope, Unset):
            scope = self.scope

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "access_token": access_token,
            }
        )
        if expires_in is not UNSET:
            field_dict["expires_in"] = expires_in
        if refresh_token is not UNSET:
            field_dict["refresh_token"] = refresh_token
        if scope is not UNSET:
            field_dict["scope"] = scope

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        access_token = d.pop("access_token")

        expires_in = d.pop("expires_in", UNSET)

        refresh_token = d.pop("refresh_token", UNSET)

        scope = cast(List[str], d.pop("scope", UNSET))

        token_response = cls(
            access_token=access_token,
            expires_in=expires_in,
            refresh_token=refresh_token,
            scope=scope,
        )

        token_response.additional_properties = d
        return token_response

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

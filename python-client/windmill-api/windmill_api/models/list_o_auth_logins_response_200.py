from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset
from typing import cast, List






T = TypeVar("T", bound="ListOAuthLoginsResponse200")


@_attrs_define
class ListOAuthLoginsResponse200:
    """ 
        Attributes:
            oauth (List[str]):
            saml (Union[Unset, str]):
     """

    oauth: List[str]
    saml: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        oauth = self.oauth




        saml = self.saml

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "oauth": oauth,
        })
        if saml is not UNSET:
            field_dict["saml"] = saml

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        oauth = cast(List[str], d.pop("oauth"))


        saml = d.pop("saml", UNSET)

        list_o_auth_logins_response_200 = cls(
            oauth=oauth,
            saml=saml,
        )

        list_o_auth_logins_response_200.additional_properties = d
        return list_o_auth_logins_response_200

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

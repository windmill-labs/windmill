from typing import Any, Dict, List, Type, TypeVar

import attr

T = TypeVar("T", bound="GetGranularAclsResponse200")


@attr.s(auto_attribs=True)
class GetGranularAclsResponse200:
    """ """

    additional_properties: Dict[str, bool] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        get_granular_acls_response_200 = cls()

        get_granular_acls_response_200.additional_properties = d
        return get_granular_acls_response_200

    @property
    def additional_keys(self) -> List[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> bool:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: bool) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties

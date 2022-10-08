from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..types import UNSET, Unset

T = TypeVar("T", bound="AddGranularAclsJsonBody")


@attr.s(auto_attribs=True)
class AddGranularAclsJsonBody:
    """
    Attributes:
        owner (str):
        write (Union[Unset, bool]):
    """

    owner: str
    write: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        owner = self.owner
        write = self.write

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "owner": owner,
            }
        )
        if write is not UNSET:
            field_dict["write"] = write

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        owner = d.pop("owner")

        write = d.pop("write", UNSET)

        add_granular_acls_json_body = cls(
            owner=owner,
            write=write,
        )

        add_granular_acls_json_body.additional_properties = d
        return add_granular_acls_json_body

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

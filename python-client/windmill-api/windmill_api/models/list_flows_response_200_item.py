from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset






T = TypeVar("T", bound="ListFlowsResponse200Item")


@_attrs_define
class ListFlowsResponse200Item:
    """ 
        Attributes:
            has_draft (Union[Unset, bool]):
            draft_only (Union[Unset, bool]):
     """

    has_draft: Union[Unset, bool] = UNSET
    draft_only: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        has_draft = self.has_draft
        draft_only = self.draft_only

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if has_draft is not UNSET:
            field_dict["has_draft"] = has_draft
        if draft_only is not UNSET:
            field_dict["draft_only"] = draft_only

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        has_draft = d.pop("has_draft", UNSET)

        draft_only = d.pop("draft_only", UNSET)

        list_flows_response_200_item = cls(
            has_draft=has_draft,
            draft_only=draft_only,
        )

        list_flows_response_200_item.additional_properties = d
        return list_flows_response_200_item

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

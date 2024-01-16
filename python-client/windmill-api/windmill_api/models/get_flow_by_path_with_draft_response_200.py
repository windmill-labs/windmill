from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from ..types import UNSET, Unset
from typing import Dict

if TYPE_CHECKING:
  from ..models.get_flow_by_path_with_draft_response_200_draft import GetFlowByPathWithDraftResponse200Draft





T = TypeVar("T", bound="GetFlowByPathWithDraftResponse200")


@_attrs_define
class GetFlowByPathWithDraftResponse200:
    """ 
        Attributes:
            draft (Union[Unset, GetFlowByPathWithDraftResponse200Draft]):
     """

    draft: Union[Unset, 'GetFlowByPathWithDraftResponse200Draft'] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.get_flow_by_path_with_draft_response_200_draft import GetFlowByPathWithDraftResponse200Draft
        draft: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.draft, Unset):
            draft = self.draft.to_dict()


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if draft is not UNSET:
            field_dict["draft"] = draft

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.get_flow_by_path_with_draft_response_200_draft import GetFlowByPathWithDraftResponse200Draft
        d = src_dict.copy()
        _draft = d.pop("draft", UNSET)
        draft: Union[Unset, GetFlowByPathWithDraftResponse200Draft]
        if isinstance(_draft,  Unset):
            draft = UNSET
        else:
            draft = GetFlowByPathWithDraftResponse200Draft.from_dict(_draft)




        get_flow_by_path_with_draft_response_200 = cls(
            draft=draft,
        )

        get_flow_by_path_with_draft_response_200.additional_properties = d
        return get_flow_by_path_with_draft_response_200

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

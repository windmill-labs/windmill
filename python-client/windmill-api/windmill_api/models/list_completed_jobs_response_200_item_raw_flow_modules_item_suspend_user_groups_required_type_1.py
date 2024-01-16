from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from ..models.list_completed_jobs_response_200_item_raw_flow_modules_item_suspend_user_groups_required_type_1_type import ListCompletedJobsResponse200ItemRawFlowModulesItemSuspendUserGroupsRequiredType1Type






T = TypeVar("T", bound="ListCompletedJobsResponse200ItemRawFlowModulesItemSuspendUserGroupsRequiredType1")


@_attrs_define
class ListCompletedJobsResponse200ItemRawFlowModulesItemSuspendUserGroupsRequiredType1:
    """ 
        Attributes:
            expr (str):
            type (ListCompletedJobsResponse200ItemRawFlowModulesItemSuspendUserGroupsRequiredType1Type):
     """

    expr: str
    type: ListCompletedJobsResponse200ItemRawFlowModulesItemSuspendUserGroupsRequiredType1Type
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        expr = self.expr
        type = self.type.value


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "expr": expr,
            "type": type,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        expr = d.pop("expr")

        type = ListCompletedJobsResponse200ItemRawFlowModulesItemSuspendUserGroupsRequiredType1Type(d.pop("type"))




        list_completed_jobs_response_200_item_raw_flow_modules_item_suspend_user_groups_required_type_1 = cls(
            expr=expr,
            type=type,
        )

        list_completed_jobs_response_200_item_raw_flow_modules_item_suspend_user_groups_required_type_1.additional_properties = d
        return list_completed_jobs_response_200_item_raw_flow_modules_item_suspend_user_groups_required_type_1

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

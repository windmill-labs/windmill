from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset
from ..models.queued_job_flow_status_modules_item_branch_chosen_type import QueuedJobFlowStatusModulesItemBranchChosenType






T = TypeVar("T", bound="QueuedJobFlowStatusModulesItemBranchChosen")


@_attrs_define
class QueuedJobFlowStatusModulesItemBranchChosen:
    """ 
        Attributes:
            type (QueuedJobFlowStatusModulesItemBranchChosenType):
            branch (Union[Unset, int]):
     """

    type: QueuedJobFlowStatusModulesItemBranchChosenType
    branch: Union[Unset, int] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        type = self.type.value

        branch = self.branch

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "type": type,
        })
        if branch is not UNSET:
            field_dict["branch"] = branch

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        type = QueuedJobFlowStatusModulesItemBranchChosenType(d.pop("type"))




        branch = d.pop("branch", UNSET)

        queued_job_flow_status_modules_item_branch_chosen = cls(
            type=type,
            branch=branch,
        )

        queued_job_flow_status_modules_item_branch_chosen.additional_properties = d
        return queued_job_flow_status_modules_item_branch_chosen

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

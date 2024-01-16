from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset






T = TypeVar("T", bound="RunFlowPreviewJsonBodyRestartedFrom")


@_attrs_define
class RunFlowPreviewJsonBodyRestartedFrom:
    """ 
        Attributes:
            flow_job_id (Union[Unset, str]):
            step_id (Union[Unset, str]):
            branch_or_iteration_n (Union[Unset, int]):
     """

    flow_job_id: Union[Unset, str] = UNSET
    step_id: Union[Unset, str] = UNSET
    branch_or_iteration_n: Union[Unset, int] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        flow_job_id = self.flow_job_id
        step_id = self.step_id
        branch_or_iteration_n = self.branch_or_iteration_n

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if flow_job_id is not UNSET:
            field_dict["flow_job_id"] = flow_job_id
        if step_id is not UNSET:
            field_dict["step_id"] = step_id
        if branch_or_iteration_n is not UNSET:
            field_dict["branch_or_iteration_n"] = branch_or_iteration_n

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        flow_job_id = d.pop("flow_job_id", UNSET)

        step_id = d.pop("step_id", UNSET)

        branch_or_iteration_n = d.pop("branch_or_iteration_n", UNSET)

        run_flow_preview_json_body_restarted_from = cls(
            flow_job_id=flow_job_id,
            step_id=step_id,
            branch_or_iteration_n=branch_or_iteration_n,
        )

        run_flow_preview_json_body_restarted_from.additional_properties = d
        return run_flow_preview_json_body_restarted_from

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

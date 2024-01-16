from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from typing import cast, List
from typing import Dict
from ..models.queued_job_flow_status_failure_module_type import QueuedJobFlowStatusFailureModuleType
from ..types import UNSET, Unset

if TYPE_CHECKING:
  from ..models.queued_job_flow_status_failure_module_branch_chosen import QueuedJobFlowStatusFailureModuleBranchChosen
  from ..models.queued_job_flow_status_failure_module_iterator import QueuedJobFlowStatusFailureModuleIterator
  from ..models.queued_job_flow_status_failure_module_approvers_item import QueuedJobFlowStatusFailureModuleApproversItem
  from ..models.queued_job_flow_status_failure_module_branchall import QueuedJobFlowStatusFailureModuleBranchall





T = TypeVar("T", bound="QueuedJobFlowStatusFailureModule")


@_attrs_define
class QueuedJobFlowStatusFailureModule:
    """ 
        Attributes:
            type (QueuedJobFlowStatusFailureModuleType):
            id (Union[Unset, str]):
            job (Union[Unset, str]):
            count (Union[Unset, int]):
            iterator (Union[Unset, QueuedJobFlowStatusFailureModuleIterator]):
            flow_jobs (Union[Unset, List[str]]):
            branch_chosen (Union[Unset, QueuedJobFlowStatusFailureModuleBranchChosen]):
            branchall (Union[Unset, QueuedJobFlowStatusFailureModuleBranchall]):
            approvers (Union[Unset, List['QueuedJobFlowStatusFailureModuleApproversItem']]):
            parent_module (Union[Unset, str]):
     """

    type: QueuedJobFlowStatusFailureModuleType
    id: Union[Unset, str] = UNSET
    job: Union[Unset, str] = UNSET
    count: Union[Unset, int] = UNSET
    iterator: Union[Unset, 'QueuedJobFlowStatusFailureModuleIterator'] = UNSET
    flow_jobs: Union[Unset, List[str]] = UNSET
    branch_chosen: Union[Unset, 'QueuedJobFlowStatusFailureModuleBranchChosen'] = UNSET
    branchall: Union[Unset, 'QueuedJobFlowStatusFailureModuleBranchall'] = UNSET
    approvers: Union[Unset, List['QueuedJobFlowStatusFailureModuleApproversItem']] = UNSET
    parent_module: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.queued_job_flow_status_failure_module_branch_chosen import QueuedJobFlowStatusFailureModuleBranchChosen
        from ..models.queued_job_flow_status_failure_module_iterator import QueuedJobFlowStatusFailureModuleIterator
        from ..models.queued_job_flow_status_failure_module_approvers_item import QueuedJobFlowStatusFailureModuleApproversItem
        from ..models.queued_job_flow_status_failure_module_branchall import QueuedJobFlowStatusFailureModuleBranchall
        type = self.type.value

        id = self.id
        job = self.job
        count = self.count
        iterator: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.iterator, Unset):
            iterator = self.iterator.to_dict()

        flow_jobs: Union[Unset, List[str]] = UNSET
        if not isinstance(self.flow_jobs, Unset):
            flow_jobs = self.flow_jobs




        branch_chosen: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.branch_chosen, Unset):
            branch_chosen = self.branch_chosen.to_dict()

        branchall: Union[Unset, Dict[str, Any]] = UNSET
        if not isinstance(self.branchall, Unset):
            branchall = self.branchall.to_dict()

        approvers: Union[Unset, List[Dict[str, Any]]] = UNSET
        if not isinstance(self.approvers, Unset):
            approvers = []
            for approvers_item_data in self.approvers:
                approvers_item = approvers_item_data.to_dict()

                approvers.append(approvers_item)




        parent_module = self.parent_module

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "type": type,
        })
        if id is not UNSET:
            field_dict["id"] = id
        if job is not UNSET:
            field_dict["job"] = job
        if count is not UNSET:
            field_dict["count"] = count
        if iterator is not UNSET:
            field_dict["iterator"] = iterator
        if flow_jobs is not UNSET:
            field_dict["flow_jobs"] = flow_jobs
        if branch_chosen is not UNSET:
            field_dict["branch_chosen"] = branch_chosen
        if branchall is not UNSET:
            field_dict["branchall"] = branchall
        if approvers is not UNSET:
            field_dict["approvers"] = approvers
        if parent_module is not UNSET:
            field_dict["parent_module"] = parent_module

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.queued_job_flow_status_failure_module_branch_chosen import QueuedJobFlowStatusFailureModuleBranchChosen
        from ..models.queued_job_flow_status_failure_module_iterator import QueuedJobFlowStatusFailureModuleIterator
        from ..models.queued_job_flow_status_failure_module_approvers_item import QueuedJobFlowStatusFailureModuleApproversItem
        from ..models.queued_job_flow_status_failure_module_branchall import QueuedJobFlowStatusFailureModuleBranchall
        d = src_dict.copy()
        type = QueuedJobFlowStatusFailureModuleType(d.pop("type"))




        id = d.pop("id", UNSET)

        job = d.pop("job", UNSET)

        count = d.pop("count", UNSET)

        _iterator = d.pop("iterator", UNSET)
        iterator: Union[Unset, QueuedJobFlowStatusFailureModuleIterator]
        if isinstance(_iterator,  Unset):
            iterator = UNSET
        else:
            iterator = QueuedJobFlowStatusFailureModuleIterator.from_dict(_iterator)




        flow_jobs = cast(List[str], d.pop("flow_jobs", UNSET))


        _branch_chosen = d.pop("branch_chosen", UNSET)
        branch_chosen: Union[Unset, QueuedJobFlowStatusFailureModuleBranchChosen]
        if isinstance(_branch_chosen,  Unset):
            branch_chosen = UNSET
        else:
            branch_chosen = QueuedJobFlowStatusFailureModuleBranchChosen.from_dict(_branch_chosen)




        _branchall = d.pop("branchall", UNSET)
        branchall: Union[Unset, QueuedJobFlowStatusFailureModuleBranchall]
        if isinstance(_branchall,  Unset):
            branchall = UNSET
        else:
            branchall = QueuedJobFlowStatusFailureModuleBranchall.from_dict(_branchall)




        approvers = []
        _approvers = d.pop("approvers", UNSET)
        for approvers_item_data in (_approvers or []):
            approvers_item = QueuedJobFlowStatusFailureModuleApproversItem.from_dict(approvers_item_data)



            approvers.append(approvers_item)


        parent_module = d.pop("parent_module", UNSET)

        queued_job_flow_status_failure_module = cls(
            type=type,
            id=id,
            job=job,
            count=count,
            iterator=iterator,
            flow_jobs=flow_jobs,
            branch_chosen=branch_chosen,
            branchall=branchall,
            approvers=approvers,
            parent_module=parent_module,
        )

        queued_job_flow_status_failure_module.additional_properties = d
        return queued_job_flow_status_failure_module

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

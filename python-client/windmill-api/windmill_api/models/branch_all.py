from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from typing import cast, List
from typing import Dict
from ..types import UNSET, Unset
from ..models.branch_all_type import BranchAllType

if TYPE_CHECKING:
  from ..models.branch_all_branches_item import BranchAllBranchesItem





T = TypeVar("T", bound="BranchAll")


@_attrs_define
class BranchAll:
    """ 
        Attributes:
            branches (List['BranchAllBranchesItem']):
            type (BranchAllType):
            parallel (Union[Unset, bool]):
     """

    branches: List['BranchAllBranchesItem']
    type: BranchAllType
    parallel: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.branch_all_branches_item import BranchAllBranchesItem
        branches = []
        for branches_item_data in self.branches:
            branches_item = branches_item_data.to_dict()

            branches.append(branches_item)




        type = self.type.value

        parallel = self.parallel

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "branches": branches,
            "type": type,
        })
        if parallel is not UNSET:
            field_dict["parallel"] = parallel

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.branch_all_branches_item import BranchAllBranchesItem
        d = src_dict.copy()
        branches = []
        _branches = d.pop("branches")
        for branches_item_data in (_branches):
            branches_item = BranchAllBranchesItem.from_dict(branches_item_data)



            branches.append(branches_item)


        type = BranchAllType(d.pop("type"))




        parallel = d.pop("parallel", UNSET)

        branch_all = cls(
            branches=branches,
            type=type,
            parallel=parallel,
        )

        branch_all.additional_properties = d
        return branch_all

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

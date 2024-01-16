from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from ..models.branch_one_type import BranchOneType
from typing import cast
from typing import cast, List
from typing import Dict

if TYPE_CHECKING:
  from ..models.branch_one_branches_item import BranchOneBranchesItem
  from ..models.branch_one_default_item import BranchOneDefaultItem





T = TypeVar("T", bound="BranchOne")


@_attrs_define
class BranchOne:
    """ 
        Attributes:
            branches (List['BranchOneBranchesItem']):
            default (List['BranchOneDefaultItem']):
            type (BranchOneType):
     """

    branches: List['BranchOneBranchesItem']
    default: List['BranchOneDefaultItem']
    type: BranchOneType
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.branch_one_branches_item import BranchOneBranchesItem
        from ..models.branch_one_default_item import BranchOneDefaultItem
        branches = []
        for branches_item_data in self.branches:
            branches_item = branches_item_data.to_dict()

            branches.append(branches_item)




        default = []
        for default_item_data in self.default:
            default_item = default_item_data.to_dict()

            default.append(default_item)




        type = self.type.value


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "branches": branches,
            "default": default,
            "type": type,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.branch_one_branches_item import BranchOneBranchesItem
        from ..models.branch_one_default_item import BranchOneDefaultItem
        d = src_dict.copy()
        branches = []
        _branches = d.pop("branches")
        for branches_item_data in (_branches):
            branches_item = BranchOneBranchesItem.from_dict(branches_item_data)



            branches.append(branches_item)


        default = []
        _default = d.pop("default")
        for default_item_data in (_default):
            default_item = BranchOneDefaultItem.from_dict(default_item_data)



            default.append(default_item)


        type = BranchOneType(d.pop("type"))




        branch_one = cls(
            branches=branches,
            default=default,
            type=type,
        )

        branch_one.additional_properties = d
        return branch_one

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

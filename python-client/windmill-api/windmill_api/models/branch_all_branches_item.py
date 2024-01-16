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

if TYPE_CHECKING:
  from ..models.branch_all_branches_item_modules_item import BranchAllBranchesItemModulesItem





T = TypeVar("T", bound="BranchAllBranchesItem")


@_attrs_define
class BranchAllBranchesItem:
    """ 
        Attributes:
            modules (List['BranchAllBranchesItemModulesItem']):
            summary (Union[Unset, str]):
            skip_failure (Union[Unset, bool]):
     """

    modules: List['BranchAllBranchesItemModulesItem']
    summary: Union[Unset, str] = UNSET
    skip_failure: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.branch_all_branches_item_modules_item import BranchAllBranchesItemModulesItem
        modules = []
        for modules_item_data in self.modules:
            modules_item = modules_item_data.to_dict()

            modules.append(modules_item)




        summary = self.summary
        skip_failure = self.skip_failure

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "modules": modules,
        })
        if summary is not UNSET:
            field_dict["summary"] = summary
        if skip_failure is not UNSET:
            field_dict["skip_failure"] = skip_failure

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.branch_all_branches_item_modules_item import BranchAllBranchesItemModulesItem
        d = src_dict.copy()
        modules = []
        _modules = d.pop("modules")
        for modules_item_data in (_modules):
            modules_item = BranchAllBranchesItemModulesItem.from_dict(modules_item_data)



            modules.append(modules_item)


        summary = d.pop("summary", UNSET)

        skip_failure = d.pop("skip_failure", UNSET)

        branch_all_branches_item = cls(
            modules=modules,
            summary=summary,
            skip_failure=skip_failure,
        )

        branch_all_branches_item.additional_properties = d
        return branch_all_branches_item

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

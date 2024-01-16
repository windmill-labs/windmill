from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from typing import cast, List
from typing import Dict
from typing import cast, Union
from ..types import UNSET, Unset
from ..models.forloop_flow_type import ForloopFlowType

if TYPE_CHECKING:
  from ..models.forloop_flow_iterator_type_1 import ForloopFlowIteratorType1
  from ..models.forloop_flow_modules_item import ForloopFlowModulesItem
  from ..models.forloop_flow_iterator_type_0 import ForloopFlowIteratorType0





T = TypeVar("T", bound="ForloopFlow")


@_attrs_define
class ForloopFlow:
    """ 
        Attributes:
            modules (List['ForloopFlowModulesItem']):
            iterator (Union['ForloopFlowIteratorType0', 'ForloopFlowIteratorType1']):
            skip_failures (bool):
            type (ForloopFlowType):
            parallel (Union[Unset, bool]):
            parallelism (Union[Unset, int]):
     """

    modules: List['ForloopFlowModulesItem']
    iterator: Union['ForloopFlowIteratorType0', 'ForloopFlowIteratorType1']
    skip_failures: bool
    type: ForloopFlowType
    parallel: Union[Unset, bool] = UNSET
    parallelism: Union[Unset, int] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.forloop_flow_iterator_type_1 import ForloopFlowIteratorType1
        from ..models.forloop_flow_modules_item import ForloopFlowModulesItem
        from ..models.forloop_flow_iterator_type_0 import ForloopFlowIteratorType0
        modules = []
        for modules_item_data in self.modules:
            modules_item = modules_item_data.to_dict()

            modules.append(modules_item)




        iterator: Dict[str, Any]

        if isinstance(self.iterator, ForloopFlowIteratorType0):
            iterator = self.iterator.to_dict()

        else:
            iterator = self.iterator.to_dict()



        skip_failures = self.skip_failures
        type = self.type.value

        parallel = self.parallel
        parallelism = self.parallelism

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "modules": modules,
            "iterator": iterator,
            "skip_failures": skip_failures,
            "type": type,
        })
        if parallel is not UNSET:
            field_dict["parallel"] = parallel
        if parallelism is not UNSET:
            field_dict["parallelism"] = parallelism

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.forloop_flow_iterator_type_1 import ForloopFlowIteratorType1
        from ..models.forloop_flow_modules_item import ForloopFlowModulesItem
        from ..models.forloop_flow_iterator_type_0 import ForloopFlowIteratorType0
        d = src_dict.copy()
        modules = []
        _modules = d.pop("modules")
        for modules_item_data in (_modules):
            modules_item = ForloopFlowModulesItem.from_dict(modules_item_data)



            modules.append(modules_item)


        def _parse_iterator(data: object) -> Union['ForloopFlowIteratorType0', 'ForloopFlowIteratorType1']:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                iterator_type_0 = ForloopFlowIteratorType0.from_dict(data)



                return iterator_type_0
            except: # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            iterator_type_1 = ForloopFlowIteratorType1.from_dict(data)



            return iterator_type_1

        iterator = _parse_iterator(d.pop("iterator"))


        skip_failures = d.pop("skip_failures")

        type = ForloopFlowType(d.pop("type"))




        parallel = d.pop("parallel", UNSET)

        parallelism = d.pop("parallelism", UNSET)

        forloop_flow = cls(
            modules=modules,
            iterator=iterator,
            skip_failures=skip_failures,
            type=type,
            parallel=parallel,
            parallelism=parallelism,
        )

        forloop_flow.additional_properties = d
        return forloop_flow

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

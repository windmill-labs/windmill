from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset






T = TypeVar("T", bound="GetJobMetricsResponse200ScalarMetricsItem")


@_attrs_define
class GetJobMetricsResponse200ScalarMetricsItem:
    """ 
        Attributes:
            value (float):
            metric_id (Union[Unset, str]):
     """

    value: float
    metric_id: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        value = self.value
        metric_id = self.metric_id

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "value": value,
        })
        if metric_id is not UNSET:
            field_dict["metric_id"] = metric_id

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        value = d.pop("value")

        metric_id = d.pop("metric_id", UNSET)

        get_job_metrics_response_200_scalar_metrics_item = cls(
            value=value,
            metric_id=metric_id,
        )

        get_job_metrics_response_200_scalar_metrics_item.additional_properties = d
        return get_job_metrics_response_200_scalar_metrics_item

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

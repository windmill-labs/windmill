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
  from ..models.timeseries_metric_values_item import TimeseriesMetricValuesItem





T = TypeVar("T", bound="TimeseriesMetric")


@_attrs_define
class TimeseriesMetric:
    """ 
        Attributes:
            values (List['TimeseriesMetricValuesItem']):
            metric_id (Union[Unset, str]):
     """

    values: List['TimeseriesMetricValuesItem']
    metric_id: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.timeseries_metric_values_item import TimeseriesMetricValuesItem
        values = []
        for values_item_data in self.values:
            values_item = values_item_data.to_dict()

            values.append(values_item)




        metric_id = self.metric_id

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "values": values,
        })
        if metric_id is not UNSET:
            field_dict["metric_id"] = metric_id

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.timeseries_metric_values_item import TimeseriesMetricValuesItem
        d = src_dict.copy()
        values = []
        _values = d.pop("values")
        for values_item_data in (_values):
            values_item = TimeseriesMetricValuesItem.from_dict(values_item_data)



            values.append(values_item)


        metric_id = d.pop("metric_id", UNSET)

        timeseries_metric = cls(
            values=values,
            metric_id=metric_id,
        )

        timeseries_metric.additional_properties = d
        return timeseries_metric

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

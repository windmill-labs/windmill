from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
import datetime
from dateutil.parser import isoparse
from ..types import UNSET, Unset






T = TypeVar("T", bound="GetJobMetricsJsonBody")


@_attrs_define
class GetJobMetricsJsonBody:
    """ 
        Attributes:
            timeseries_max_datapoints (Union[Unset, int]):
            from_timestamp (Union[Unset, datetime.datetime]):
            to_timestamp (Union[Unset, datetime.datetime]):
     """

    timeseries_max_datapoints: Union[Unset, int] = UNSET
    from_timestamp: Union[Unset, datetime.datetime] = UNSET
    to_timestamp: Union[Unset, datetime.datetime] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        timeseries_max_datapoints = self.timeseries_max_datapoints
        from_timestamp: Union[Unset, str] = UNSET
        if not isinstance(self.from_timestamp, Unset):
            from_timestamp = self.from_timestamp.isoformat()

        to_timestamp: Union[Unset, str] = UNSET
        if not isinstance(self.to_timestamp, Unset):
            to_timestamp = self.to_timestamp.isoformat()


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if timeseries_max_datapoints is not UNSET:
            field_dict["timeseries_max_datapoints"] = timeseries_max_datapoints
        if from_timestamp is not UNSET:
            field_dict["from_timestamp"] = from_timestamp
        if to_timestamp is not UNSET:
            field_dict["to_timestamp"] = to_timestamp

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        timeseries_max_datapoints = d.pop("timeseries_max_datapoints", UNSET)

        _from_timestamp = d.pop("from_timestamp", UNSET)
        from_timestamp: Union[Unset, datetime.datetime]
        if isinstance(_from_timestamp,  Unset):
            from_timestamp = UNSET
        else:
            from_timestamp = isoparse(_from_timestamp)




        _to_timestamp = d.pop("to_timestamp", UNSET)
        to_timestamp: Union[Unset, datetime.datetime]
        if isinstance(_to_timestamp,  Unset):
            to_timestamp = UNSET
        else:
            to_timestamp = isoparse(_to_timestamp)




        get_job_metrics_json_body = cls(
            timeseries_max_datapoints=timeseries_max_datapoints,
            from_timestamp=from_timestamp,
            to_timestamp=to_timestamp,
        )

        get_job_metrics_json_body.additional_properties = d
        return get_job_metrics_json_body

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

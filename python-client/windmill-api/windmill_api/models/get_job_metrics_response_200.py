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
  from ..models.get_job_metrics_response_200_scalar_metrics_item import GetJobMetricsResponse200ScalarMetricsItem
  from ..models.get_job_metrics_response_200_timeseries_metrics_item import GetJobMetricsResponse200TimeseriesMetricsItem
  from ..models.get_job_metrics_response_200_metrics_metadata_item import GetJobMetricsResponse200MetricsMetadataItem





T = TypeVar("T", bound="GetJobMetricsResponse200")


@_attrs_define
class GetJobMetricsResponse200:
    """ 
        Attributes:
            metrics_metadata (Union[Unset, List['GetJobMetricsResponse200MetricsMetadataItem']]):
            scalar_metrics (Union[Unset, List['GetJobMetricsResponse200ScalarMetricsItem']]):
            timeseries_metrics (Union[Unset, List['GetJobMetricsResponse200TimeseriesMetricsItem']]):
     """

    metrics_metadata: Union[Unset, List['GetJobMetricsResponse200MetricsMetadataItem']] = UNSET
    scalar_metrics: Union[Unset, List['GetJobMetricsResponse200ScalarMetricsItem']] = UNSET
    timeseries_metrics: Union[Unset, List['GetJobMetricsResponse200TimeseriesMetricsItem']] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.get_job_metrics_response_200_scalar_metrics_item import GetJobMetricsResponse200ScalarMetricsItem
        from ..models.get_job_metrics_response_200_timeseries_metrics_item import GetJobMetricsResponse200TimeseriesMetricsItem
        from ..models.get_job_metrics_response_200_metrics_metadata_item import GetJobMetricsResponse200MetricsMetadataItem
        metrics_metadata: Union[Unset, List[Dict[str, Any]]] = UNSET
        if not isinstance(self.metrics_metadata, Unset):
            metrics_metadata = []
            for metrics_metadata_item_data in self.metrics_metadata:
                metrics_metadata_item = metrics_metadata_item_data.to_dict()

                metrics_metadata.append(metrics_metadata_item)




        scalar_metrics: Union[Unset, List[Dict[str, Any]]] = UNSET
        if not isinstance(self.scalar_metrics, Unset):
            scalar_metrics = []
            for scalar_metrics_item_data in self.scalar_metrics:
                scalar_metrics_item = scalar_metrics_item_data.to_dict()

                scalar_metrics.append(scalar_metrics_item)




        timeseries_metrics: Union[Unset, List[Dict[str, Any]]] = UNSET
        if not isinstance(self.timeseries_metrics, Unset):
            timeseries_metrics = []
            for timeseries_metrics_item_data in self.timeseries_metrics:
                timeseries_metrics_item = timeseries_metrics_item_data.to_dict()

                timeseries_metrics.append(timeseries_metrics_item)





        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if metrics_metadata is not UNSET:
            field_dict["metrics_metadata"] = metrics_metadata
        if scalar_metrics is not UNSET:
            field_dict["scalar_metrics"] = scalar_metrics
        if timeseries_metrics is not UNSET:
            field_dict["timeseries_metrics"] = timeseries_metrics

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.get_job_metrics_response_200_scalar_metrics_item import GetJobMetricsResponse200ScalarMetricsItem
        from ..models.get_job_metrics_response_200_timeseries_metrics_item import GetJobMetricsResponse200TimeseriesMetricsItem
        from ..models.get_job_metrics_response_200_metrics_metadata_item import GetJobMetricsResponse200MetricsMetadataItem
        d = src_dict.copy()
        metrics_metadata = []
        _metrics_metadata = d.pop("metrics_metadata", UNSET)
        for metrics_metadata_item_data in (_metrics_metadata or []):
            metrics_metadata_item = GetJobMetricsResponse200MetricsMetadataItem.from_dict(metrics_metadata_item_data)



            metrics_metadata.append(metrics_metadata_item)


        scalar_metrics = []
        _scalar_metrics = d.pop("scalar_metrics", UNSET)
        for scalar_metrics_item_data in (_scalar_metrics or []):
            scalar_metrics_item = GetJobMetricsResponse200ScalarMetricsItem.from_dict(scalar_metrics_item_data)



            scalar_metrics.append(scalar_metrics_item)


        timeseries_metrics = []
        _timeseries_metrics = d.pop("timeseries_metrics", UNSET)
        for timeseries_metrics_item_data in (_timeseries_metrics or []):
            timeseries_metrics_item = GetJobMetricsResponse200TimeseriesMetricsItem.from_dict(timeseries_metrics_item_data)



            timeseries_metrics.append(timeseries_metrics_item)


        get_job_metrics_response_200 = cls(
            metrics_metadata=metrics_metadata,
            scalar_metrics=scalar_metrics,
            timeseries_metrics=timeseries_metrics,
        )

        get_job_metrics_response_200.additional_properties = d
        return get_job_metrics_response_200

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

from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset







T = TypeVar("T", bound="GetFolderUsageResponse200")


@_attrs_define
class GetFolderUsageResponse200:
    """ 
        Attributes:
            scripts (float):
            flows (float):
            apps (float):
            resources (float):
            variables (float):
            schedules (float):
     """

    scripts: float
    flows: float
    apps: float
    resources: float
    variables: float
    schedules: float
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        scripts = self.scripts
        flows = self.flows
        apps = self.apps
        resources = self.resources
        variables = self.variables
        schedules = self.schedules

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "scripts": scripts,
            "flows": flows,
            "apps": apps,
            "resources": resources,
            "variables": variables,
            "schedules": schedules,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        scripts = d.pop("scripts")

        flows = d.pop("flows")

        apps = d.pop("apps")

        resources = d.pop("resources")

        variables = d.pop("variables")

        schedules = d.pop("schedules")

        get_folder_usage_response_200 = cls(
            scripts=scripts,
            flows=flows,
            apps=apps,
            resources=resources,
            variables=variables,
            schedules=schedules,
        )

        get_folder_usage_response_200.additional_properties = d
        return get_folder_usage_response_200

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

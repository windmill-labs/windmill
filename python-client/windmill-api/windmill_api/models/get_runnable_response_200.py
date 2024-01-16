from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from ..types import UNSET, Unset






T = TypeVar("T", bound="GetRunnableResponse200")


@_attrs_define
class GetRunnableResponse200:
    """ 
        Attributes:
            workspace (str):
            endpoint_async (str):
            endpoint_sync (str):
            endpoint_openai_sync (str):
            summary (str):
            kind (str):
            description (Union[Unset, str]):
     """

    workspace: str
    endpoint_async: str
    endpoint_sync: str
    endpoint_openai_sync: str
    summary: str
    kind: str
    description: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        workspace = self.workspace
        endpoint_async = self.endpoint_async
        endpoint_sync = self.endpoint_sync
        endpoint_openai_sync = self.endpoint_openai_sync
        summary = self.summary
        kind = self.kind
        description = self.description

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "workspace": workspace,
            "endpoint_async": endpoint_async,
            "endpoint_sync": endpoint_sync,
            "endpoint_openai_sync": endpoint_openai_sync,
            "summary": summary,
            "kind": kind,
        })
        if description is not UNSET:
            field_dict["description"] = description

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        workspace = d.pop("workspace")

        endpoint_async = d.pop("endpoint_async")

        endpoint_sync = d.pop("endpoint_sync")

        endpoint_openai_sync = d.pop("endpoint_openai_sync")

        summary = d.pop("summary")

        kind = d.pop("kind")

        description = d.pop("description", UNSET)

        get_runnable_response_200 = cls(
            workspace=workspace,
            endpoint_async=endpoint_async,
            endpoint_sync=endpoint_sync,
            endpoint_openai_sync=endpoint_openai_sync,
            summary=summary,
            kind=kind,
            description=description,
        )

        get_runnable_response_200.additional_properties = d
        return get_runnable_response_200

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

from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import cast
from typing import Dict

if TYPE_CHECKING:
  from ..models.get_hub_app_by_id_response_200_app import GetHubAppByIdResponse200App





T = TypeVar("T", bound="GetHubAppByIdResponse200")


@_attrs_define
class GetHubAppByIdResponse200:
    """ 
        Attributes:
            app (GetHubAppByIdResponse200App):
     """

    app: 'GetHubAppByIdResponse200App'
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.get_hub_app_by_id_response_200_app import GetHubAppByIdResponse200App
        app = self.app.to_dict()


        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "app": app,
        })

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.get_hub_app_by_id_response_200_app import GetHubAppByIdResponse200App
        d = src_dict.copy()
        app = GetHubAppByIdResponse200App.from_dict(d.pop("app"))




        get_hub_app_by_id_response_200 = cls(
            app=app,
        )

        get_hub_app_by_id_response_200.additional_properties = d
        return get_hub_app_by_id_response_200

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

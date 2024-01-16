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
  from ..models.list_hub_apps_response_200_apps_item import ListHubAppsResponse200AppsItem





T = TypeVar("T", bound="ListHubAppsResponse200")


@_attrs_define
class ListHubAppsResponse200:
    """ 
        Attributes:
            apps (Union[Unset, List['ListHubAppsResponse200AppsItem']]):
     """

    apps: Union[Unset, List['ListHubAppsResponse200AppsItem']] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.list_hub_apps_response_200_apps_item import ListHubAppsResponse200AppsItem
        apps: Union[Unset, List[Dict[str, Any]]] = UNSET
        if not isinstance(self.apps, Unset):
            apps = []
            for apps_item_data in self.apps:
                apps_item = apps_item_data.to_dict()

                apps.append(apps_item)





        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
        })
        if apps is not UNSET:
            field_dict["apps"] = apps

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.list_hub_apps_response_200_apps_item import ListHubAppsResponse200AppsItem
        d = src_dict.copy()
        apps = []
        _apps = d.pop("apps", UNSET)
        for apps_item_data in (_apps or []):
            apps_item = ListHubAppsResponse200AppsItem.from_dict(apps_item_data)



            apps.append(apps_item)


        list_hub_apps_response_200 = cls(
            apps=apps,
        )

        list_hub_apps_response_200.additional_properties = d
        return list_hub_apps_response_200

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

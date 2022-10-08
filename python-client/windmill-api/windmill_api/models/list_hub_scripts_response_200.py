from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.list_hub_scripts_response_200_asks_item import ListHubScriptsResponse200AsksItem
from ..types import UNSET, Unset

T = TypeVar("T", bound="ListHubScriptsResponse200")


@attr.s(auto_attribs=True)
class ListHubScriptsResponse200:
    """
    Attributes:
        asks (Union[Unset, List[ListHubScriptsResponse200AsksItem]]):
    """

    asks: Union[Unset, List[ListHubScriptsResponse200AsksItem]] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        asks: Union[Unset, List[Dict[str, Any]]] = UNSET
        if not isinstance(self.asks, Unset):
            asks = []
            for asks_item_data in self.asks:
                asks_item = asks_item_data.to_dict()

                asks.append(asks_item)

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if asks is not UNSET:
            field_dict["asks"] = asks

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        asks = []
        _asks = d.pop("asks", UNSET)
        for asks_item_data in _asks or []:
            asks_item = ListHubScriptsResponse200AsksItem.from_dict(asks_item_data)

            asks.append(asks_item)

        list_hub_scripts_response_200 = cls(
            asks=asks,
        )

        list_hub_scripts_response_200.additional_properties = d
        return list_hub_scripts_response_200

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

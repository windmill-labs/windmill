from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.main_arg_signature_args_item import MainArgSignatureArgsItem
from ..types import UNSET, Unset

T = TypeVar("T", bound="MainArgSignature")


@attr.s(auto_attribs=True)
class MainArgSignature:
    """
    Attributes:
        star_args (bool):
        args (List[MainArgSignatureArgsItem]):
        star_kwargs (Union[Unset, bool]):
    """

    star_args: bool
    args: List[MainArgSignatureArgsItem]
    star_kwargs: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        star_args = self.star_args
        args = []
        for args_item_data in self.args:
            args_item = args_item_data.to_dict()

            args.append(args_item)

        star_kwargs = self.star_kwargs

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "star_args": star_args,
                "args": args,
            }
        )
        if star_kwargs is not UNSET:
            field_dict["star_kwargs"] = star_kwargs

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        star_args = d.pop("star_args")

        args = []
        _args = d.pop("args")
        for args_item_data in _args:
            args_item = MainArgSignatureArgsItem.from_dict(args_item_data)

            args.append(args_item)

        star_kwargs = d.pop("star_kwargs", UNSET)

        main_arg_signature = cls(
            star_args=star_args,
            args=args,
            star_kwargs=star_kwargs,
        )

        main_arg_signature.additional_properties = d
        return main_arg_signature

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

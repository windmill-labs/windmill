from typing import Any, Dict, List, Type, TypeVar, Union

import attr

from ..models.preview_args import PreviewArgs
from ..models.preview_language import PreviewLanguage
from ..types import UNSET, Unset

T = TypeVar("T", bound="Preview")


@attr.s(auto_attribs=True)
class Preview:
    """
    Attributes:
        content (str):
        args (PreviewArgs):
        language (PreviewLanguage):
        path (Union[Unset, str]):
    """

    content: str
    args: PreviewArgs
    language: PreviewLanguage
    path: Union[Unset, str] = UNSET
    additional_properties: Dict[str, Any] = attr.ib(init=False, factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        content = self.content
        args = self.args.to_dict()

        language = self.language.value

        path = self.path

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "content": content,
                "args": args,
                "language": language,
            }
        )
        if path is not UNSET:
            field_dict["path"] = path

        return field_dict

    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        d = src_dict.copy()
        content = d.pop("content")

        args = PreviewArgs.from_dict(d.pop("args"))

        language = PreviewLanguage(d.pop("language"))

        path = d.pop("path", UNSET)

        preview = cls(
            content=content,
            args=args,
            language=language,
            path=path,
        )

        preview.additional_properties = d
        return preview

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

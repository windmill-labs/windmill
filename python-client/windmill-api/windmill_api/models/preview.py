from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from ..models.preview_kind import PreviewKind
from typing import Dict
from ..models.preview_language import PreviewLanguage
from ..types import UNSET, Unset

if TYPE_CHECKING:
  from ..models.preview_args import PreviewArgs





T = TypeVar("T", bound="Preview")


@_attrs_define
class Preview:
    """ 
        Attributes:
            args (PreviewArgs):
            content (Union[Unset, str]):
            path (Union[Unset, str]):
            language (Union[Unset, PreviewLanguage]):
            tag (Union[Unset, str]):
            kind (Union[Unset, PreviewKind]):
            dedicated_worker (Union[Unset, bool]):
     """

    args: 'PreviewArgs'
    content: Union[Unset, str] = UNSET
    path: Union[Unset, str] = UNSET
    language: Union[Unset, PreviewLanguage] = UNSET
    tag: Union[Unset, str] = UNSET
    kind: Union[Unset, PreviewKind] = UNSET
    dedicated_worker: Union[Unset, bool] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.preview_args import PreviewArgs
        args = self.args.to_dict()

        content = self.content
        path = self.path
        language: Union[Unset, str] = UNSET
        if not isinstance(self.language, Unset):
            language = self.language.value

        tag = self.tag
        kind: Union[Unset, str] = UNSET
        if not isinstance(self.kind, Unset):
            kind = self.kind.value

        dedicated_worker = self.dedicated_worker

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "args": args,
        })
        if content is not UNSET:
            field_dict["content"] = content
        if path is not UNSET:
            field_dict["path"] = path
        if language is not UNSET:
            field_dict["language"] = language
        if tag is not UNSET:
            field_dict["tag"] = tag
        if kind is not UNSET:
            field_dict["kind"] = kind
        if dedicated_worker is not UNSET:
            field_dict["dedicated_worker"] = dedicated_worker

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.preview_args import PreviewArgs
        d = src_dict.copy()
        args = PreviewArgs.from_dict(d.pop("args"))




        content = d.pop("content", UNSET)

        path = d.pop("path", UNSET)

        _language = d.pop("language", UNSET)
        language: Union[Unset, PreviewLanguage]
        if isinstance(_language,  Unset):
            language = UNSET
        else:
            language = PreviewLanguage(_language)




        tag = d.pop("tag", UNSET)

        _kind = d.pop("kind", UNSET)
        kind: Union[Unset, PreviewKind]
        if isinstance(_kind,  Unset):
            kind = UNSET
        else:
            kind = PreviewKind(_kind)




        dedicated_worker = d.pop("dedicated_worker", UNSET)

        preview = cls(
            args=args,
            content=content,
            path=path,
            language=language,
            tag=tag,
            kind=kind,
            dedicated_worker=dedicated_worker,
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

from typing import Any, Dict, Type, TypeVar, Tuple, Optional, BinaryIO, TextIO, TYPE_CHECKING

from typing import List


from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

from typing import Union
from typing import cast
from ..models.raw_script_language import RawScriptLanguage
from typing import Dict
from ..types import UNSET, Unset
from ..models.raw_script_type import RawScriptType

if TYPE_CHECKING:
  from ..models.raw_script_input_transforms import RawScriptInputTransforms





T = TypeVar("T", bound="RawScript")


@_attrs_define
class RawScript:
    """ 
        Attributes:
            input_transforms (RawScriptInputTransforms):
            content (str):
            language (RawScriptLanguage):
            type (RawScriptType):
            path (Union[Unset, str]):
            lock (Union[Unset, str]):
            tag (Union[Unset, str]):
            concurrent_limit (Union[Unset, float]):
            concurrency_time_window_s (Union[Unset, float]):
     """

    input_transforms: 'RawScriptInputTransforms'
    content: str
    language: RawScriptLanguage
    type: RawScriptType
    path: Union[Unset, str] = UNSET
    lock: Union[Unset, str] = UNSET
    tag: Union[Unset, str] = UNSET
    concurrent_limit: Union[Unset, float] = UNSET
    concurrency_time_window_s: Union[Unset, float] = UNSET
    additional_properties: Dict[str, Any] = _attrs_field(init=False, factory=dict)


    def to_dict(self) -> Dict[str, Any]:
        from ..models.raw_script_input_transforms import RawScriptInputTransforms
        input_transforms = self.input_transforms.to_dict()

        content = self.content
        language = self.language.value

        type = self.type.value

        path = self.path
        lock = self.lock
        tag = self.tag
        concurrent_limit = self.concurrent_limit
        concurrency_time_window_s = self.concurrency_time_window_s

        field_dict: Dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({
            "input_transforms": input_transforms,
            "content": content,
            "language": language,
            "type": type,
        })
        if path is not UNSET:
            field_dict["path"] = path
        if lock is not UNSET:
            field_dict["lock"] = lock
        if tag is not UNSET:
            field_dict["tag"] = tag
        if concurrent_limit is not UNSET:
            field_dict["concurrent_limit"] = concurrent_limit
        if concurrency_time_window_s is not UNSET:
            field_dict["concurrency_time_window_s"] = concurrency_time_window_s

        return field_dict



    @classmethod
    def from_dict(cls: Type[T], src_dict: Dict[str, Any]) -> T:
        from ..models.raw_script_input_transforms import RawScriptInputTransforms
        d = src_dict.copy()
        input_transforms = RawScriptInputTransforms.from_dict(d.pop("input_transforms"))




        content = d.pop("content")

        language = RawScriptLanguage(d.pop("language"))




        type = RawScriptType(d.pop("type"))




        path = d.pop("path", UNSET)

        lock = d.pop("lock", UNSET)

        tag = d.pop("tag", UNSET)

        concurrent_limit = d.pop("concurrent_limit", UNSET)

        concurrency_time_window_s = d.pop("concurrency_time_window_s", UNSET)

        raw_script = cls(
            input_transforms=input_transforms,
            content=content,
            language=language,
            type=type,
            path=path,
            lock=lock,
            tag=tag,
            concurrent_limit=concurrent_limit,
            concurrency_time_window_s=concurrency_time_window_s,
        )

        raw_script.additional_properties = d
        return raw_script

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

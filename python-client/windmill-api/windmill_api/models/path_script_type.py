from enum import Enum


class PathScriptType(str, Enum):
    SCRIPT = "script"

    def __str__(self) -> str:
        return str(self.value)

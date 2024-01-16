from enum import Enum

class RawScriptType(str, Enum):
    RAWSCRIPT = "rawscript"

    def __str__(self) -> str:
        return str(self.value)

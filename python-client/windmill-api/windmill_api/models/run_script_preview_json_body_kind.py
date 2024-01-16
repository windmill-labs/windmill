from enum import Enum

class RunScriptPreviewJsonBodyKind(str, Enum):
    CODE = "code"
    HTTP = "http"
    IDENTITY = "identity"

    def __str__(self) -> str:
        return str(self.value)

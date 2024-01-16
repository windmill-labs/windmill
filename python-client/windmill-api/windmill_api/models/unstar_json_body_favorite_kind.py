from enum import Enum

class UnstarJsonBodyFavoriteKind(str, Enum):
    APP = "app"
    FLOW = "flow"
    RAW_APP = "raw_app"
    SCRIPT = "script"

    def __str__(self) -> str:
        return str(self.value)

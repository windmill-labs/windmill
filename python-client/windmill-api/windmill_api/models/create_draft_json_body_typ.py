from enum import Enum

class CreateDraftJsonBodyTyp(str, Enum):
    APP = "app"
    FLOW = "flow"
    SCRIPT = "script"

    def __str__(self) -> str:
        return str(self.value)

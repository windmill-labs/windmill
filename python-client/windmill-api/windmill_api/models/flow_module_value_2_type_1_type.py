from enum import Enum

class FlowModuleValue2Type1Type(str, Enum):
    SCRIPT = "script"

    def __str__(self) -> str:
        return str(self.value)

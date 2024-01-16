from enum import Enum

class FlowModuleValue2Type0Type(str, Enum):
    RAWSCRIPT = "rawscript"

    def __str__(self) -> str:
        return str(self.value)

from enum import Enum

class FlowModuleValue2Type2Type(str, Enum):
    FLOW = "flow"

    def __str__(self) -> str:
        return str(self.value)

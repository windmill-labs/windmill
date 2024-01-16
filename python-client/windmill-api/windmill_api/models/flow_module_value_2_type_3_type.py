from enum import Enum

class FlowModuleValue2Type3Type(str, Enum):
    FORLOOPFLOW = "forloopflow"

    def __str__(self) -> str:
        return str(self.value)

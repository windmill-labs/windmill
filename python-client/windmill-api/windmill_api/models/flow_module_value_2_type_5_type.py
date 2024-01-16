from enum import Enum

class FlowModuleValue2Type5Type(str, Enum):
    BRANCHALL = "branchall"

    def __str__(self) -> str:
        return str(self.value)

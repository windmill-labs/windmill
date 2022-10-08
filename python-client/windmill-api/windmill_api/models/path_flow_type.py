from enum import Enum


class PathFlowType(str, Enum):
    FLOW = "flow"

    def __str__(self) -> str:
        return str(self.value)

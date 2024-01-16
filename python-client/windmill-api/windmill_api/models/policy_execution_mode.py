from enum import Enum

class PolicyExecutionMode(str, Enum):
    ANONYMOUS = "anonymous"
    PUBLISHER = "publisher"
    VIEWER = "viewer"

    def __str__(self) -> str:
        return str(self.value)

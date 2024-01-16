from enum import Enum

class ListQueueResponse200ItemFlowStatusFailureModuleBranchChosenType(str, Enum):
    BRANCH = "branch"
    DEFAULT = "default"

    def __str__(self) -> str:
        return str(self.value)

from enum import Enum

class GetCompletedJobResponse200FlowStatusModulesItemBranchChosenType(str, Enum):
    BRANCH = "branch"
    DEFAULT = "default"

    def __str__(self) -> str:
        return str(self.value)

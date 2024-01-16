from enum import Enum

class BranchAllType(str, Enum):
    BRANCHALL = "branchall"

    def __str__(self) -> str:
        return str(self.value)

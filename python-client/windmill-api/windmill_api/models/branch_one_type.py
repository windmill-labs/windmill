from enum import Enum

class BranchOneType(str, Enum):
    BRANCHONE = "branchone"

    def __str__(self) -> str:
        return str(self.value)

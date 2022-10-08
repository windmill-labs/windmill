from enum import Enum


class ListCompletedJobsResponse200ItemRawFlowModulesItemValueType0Language(str, Enum):
    DENO = "deno"
    PYTHON3 = "python3"
    GO = "go"

    def __str__(self) -> str:
        return str(self.value)

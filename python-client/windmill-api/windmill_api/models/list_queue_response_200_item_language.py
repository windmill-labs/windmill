from enum import Enum


class ListQueueResponse200ItemLanguage(str, Enum):
    PYTHON3 = "python3"
    DENO = "deno"
    GO = "go"

    def __str__(self) -> str:
        return str(self.value)

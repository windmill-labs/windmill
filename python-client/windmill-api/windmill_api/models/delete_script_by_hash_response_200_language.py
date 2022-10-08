from enum import Enum


class DeleteScriptByHashResponse200Language(str, Enum):
    PYTHON3 = "python3"
    DENO = "deno"
    GO = "go"

    def __str__(self) -> str:
        return str(self.value)

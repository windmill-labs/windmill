from enum import Enum


class CreateScriptJsonBodyKind(str, Enum):
    SCRIPT = "script"
    FAILURE = "failure"
    TRIGGER = "trigger"
    COMMAND = "command"

    def __str__(self) -> str:
        return str(self.value)

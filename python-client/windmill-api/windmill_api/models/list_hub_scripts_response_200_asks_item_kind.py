from enum import Enum


class ListHubScriptsResponse200AsksItemKind(str, Enum):
    SCRIPT = "script"
    FAILURE = "failure"
    TRIGGER = "trigger"
    COMMAND = "command"

    def __str__(self) -> str:
        return str(self.value)

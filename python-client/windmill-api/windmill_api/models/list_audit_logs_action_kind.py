from enum import Enum


class ListAuditLogsActionKind(str, Enum):
    CREATE = "Create"
    UPDATE = "Update"
    DELETE = "Delete"
    EXECUTE = "Execute"

    def __str__(self) -> str:
        return str(self.value)

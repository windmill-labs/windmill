from enum import Enum

class ListAuditLogsActionKind(str, Enum):
    CREATE = "Create"
    DELETE = "Delete"
    EXECUTE = "Execute"
    UPDATE = "Update"

    def __str__(self) -> str:
        return str(self.value)

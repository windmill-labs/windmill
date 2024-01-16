from enum import Enum

class ListAuditLogsResponse200ItemActionKind(str, Enum):
    CREATED = "Created"
    DELETE = "Delete"
    EXECUTE = "Execute"
    UPDATED = "Updated"

    def __str__(self) -> str:
        return str(self.value)

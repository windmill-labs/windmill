from enum import Enum


class ListAuditLogsResponse200ItemActionKind(str, Enum):
    CREATED = "Created"
    UPDATED = "Updated"
    DELETE = "Delete"
    EXECUTE = "Execute"

    def __str__(self) -> str:
        return str(self.value)

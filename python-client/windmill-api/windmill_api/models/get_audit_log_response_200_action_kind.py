from enum import Enum

class GetAuditLogResponse200ActionKind(str, Enum):
    CREATED = "Created"
    DELETE = "Delete"
    EXECUTE = "Execute"
    UPDATED = "Updated"

    def __str__(self) -> str:
        return str(self.value)

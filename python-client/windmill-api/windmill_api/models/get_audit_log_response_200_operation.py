from enum import Enum


class GetAuditLogResponse200Operation(str, Enum):
    JOBS_RUN = "jobs.run"
    SCRIPTS_CREATE = "scripts.create"
    SCRIPTS_UPDATE = "scripts.update"
    USERS_CREATE = "users.create"
    USERS_DELETE = "users.delete"
    USERS_SETPASSWORD = "users.setpassword"
    USERS_UPDATE = "users.update"
    USERS_LOGIN = "users.login"
    USERS_TOKEN_CREATE = "users.token.create"
    USERS_TOKEN_DELETE = "users.token.delete"
    VARIABLES_CREATE = "variables.create"
    VARIABLES_DELETE = "variables.delete"
    VARIABLES_UPDATE = "variables.update"

    def __str__(self) -> str:
        return str(self.value)

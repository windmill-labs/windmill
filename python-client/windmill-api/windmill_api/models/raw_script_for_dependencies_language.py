from enum import Enum

class RawScriptForDependenciesLanguage(str, Enum):
    BASH = "bash"
    BIGQUERY = "bigquery"
    BUN = "bun"
    DENO = "deno"
    GO = "go"
    GRAPHQL = "graphql"
    MSSQL = "mssql"
    MYSQL = "mysql"
    NATIVETS = "nativets"
    POSTGRESQL = "postgresql"
    POWERSHELL = "powershell"
    PYTHON3 = "python3"
    SNOWFLAKE = "snowflake"

    def __str__(self) -> str:
        return str(self.value)

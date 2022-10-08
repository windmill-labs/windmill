from enum import Enum


class DenoToJsonschemaResponse200ArgsItemTypType0(str, Enum):
    FLOAT = "float"
    INT = "int"
    BOOL = "bool"
    EMAIL = "email"
    UNKNOWN = "unknown"
    BYTES = "bytes"
    DICT = "dict"
    DATETIME = "datetime"
    SQL = "sql"

    def __str__(self) -> str:
        return str(self.value)

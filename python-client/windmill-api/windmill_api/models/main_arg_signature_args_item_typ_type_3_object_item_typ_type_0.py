from enum import Enum

class MainArgSignatureArgsItemTypType3ObjectItemTypType0(str, Enum):
    BOOL = "bool"
    BYTES = "bytes"
    DATETIME = "datetime"
    DICT = "dict"
    EMAIL = "email"
    FLOAT = "float"
    INT = "int"
    SQL = "sql"
    UNKNOWN = "unknown"

    def __str__(self) -> str:
        return str(self.value)

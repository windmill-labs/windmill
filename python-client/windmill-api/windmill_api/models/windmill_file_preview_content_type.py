from enum import Enum

class WindmillFilePreviewContentType(str, Enum):
    CSV = "Csv"
    PARQUET = "Parquet"
    RAWTEXT = "RawText"
    UNKNOWN = "Unknown"

    def __str__(self) -> str:
        return str(self.value)

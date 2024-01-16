from enum import Enum

class GetSettingsResponse200LargeFileStorageType(str, Enum):
    S3STORAGE = "S3Storage"

    def __str__(self) -> str:
        return str(self.value)

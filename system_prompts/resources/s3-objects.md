# S3 Object Operations

Windmill provides built-in support for S3-compatible storage operations.

## S3Object Type

The S3Object type represents a file in S3 storage:

```typescript
type S3Object = {
  s3: string;  // Path within the bucket
}
```

## TypeScript Operations

```typescript
import * as wmill from 'windmill-client'

// Load file content from S3
const content: Uint8Array = await wmill.loadS3File(s3object)

// Load file as stream
const blob: Blob = await wmill.loadS3FileStream(s3object)

// Write file to S3
const result: S3Object = await wmill.writeS3File(
  s3object,      // Target path (or undefined to auto-generate)
  fileContent,   // string or Blob
  s3ResourcePath // Optional: specific S3 resource to use
)
```

## Python Operations

```python
import wmill

# Load file content from S3
content: bytes = wmill.load_s3_file(s3object)

# Load file as stream reader
reader: BufferedReader = wmill.load_s3_file_reader(s3object)

# Write file to S3
result: S3Object = wmill.write_s3_file(
    s3object,           # Target path (or None to auto-generate)
    file_content,       # bytes or BufferedReader
    s3_resource_path,   # Optional: specific S3 resource
    content_type,       # Optional: MIME type
    content_disposition # Optional: Content-Disposition header
)
```

## DuckDB S3 Operations

Read files directly from S3 in DuckDB queries:

```sql
-- Read CSV from default S3 storage
SELECT * FROM read_csv('s3:///path/to/file.csv');

-- Read from named storage
SELECT * FROM read_csv('s3://storage_name/path/to/file.csv');

-- Read Parquet files
SELECT * FROM read_parquet('s3:///data/*.parquet');

-- Read JSON files
SELECT * FROM read_json('s3:///path/to/file.json');
```

## Flow Input Schema

To accept an S3 object as flow input:

```json
{
  "type": "object",
  "properties": {
    "file": {
      "type": "object",
      "format": "resource-s3_object",
      "description": "File to process"
    }
  }
}
```

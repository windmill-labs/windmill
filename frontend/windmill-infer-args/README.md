# windmill-infer-args

A TypeScript library for inferring arguments from code snippets in various languages for Windmill workflows.

## Installation

```bash
npm install windmill-infer-args
```

## Usage

```typescript
import { inferArgs } from 'windmill-infer-args'

// Example schema
const schema = {
	properties: {},
	required: []
}

async function main() {
	// Infer arguments from Python code
	const pythonCode = `
def main(name: str, age: int = 30):
    print(f"Hello, {name}! You are {age} years old.")
  `

	const result = await inferArgs('python3', pythonCode, schema)
	console.log(schema)
	// Output will contain properties for 'name' and 'age' with proper types
	// 'name' will be in the required array since it has no default value
}

main()
```

## Supported Languages

This package supports inferring arguments from code in the following languages:

- Python (`python3`)
- TypeScript/JavaScript (`deno`, `nativets`, `bun`, `bunnative`)
- SQL variants (`postgresql`, `mysql`, `bigquery`, `snowflake`, `mssql`, `oracledb`)
- GraphQL (`graphql`)
- Go (`go`)
- Bash (`bash`)
- PowerShell (`powershell`)
- PHP (`php`)
- Rust (`rust`)
- Ansible (`ansible`)
- C# (`csharp`)
- Nu Shell (`nu`)
- Java (`java`)

## API

### inferArgs

Infers arguments from code and populates a schema object with the inferred types.

```typescript
async function inferArgs(
	language: SupportedLanguage | 'bunnative' | undefined,
	code: string,
	schema: Schema,
	mainOverride?: string
): Promise<{
	no_main_func: boolean | null
	has_preprocessor: boolean | null
} | null>
```

Parameters:

- `language`: The programming language of the code
- `code`: The source code to infer arguments from
- `schema`: A JSON schema object to populate with inferred types
- `mainOverride`: (Optional) Override for the main function name

Returns:

- An object with information about the parsed code or `null` if the language is not supported

## License

ISC

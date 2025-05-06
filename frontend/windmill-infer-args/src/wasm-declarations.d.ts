// Declaration for wasm imports
declare module '*.wasm?url' {
	const content: string
	export default content
}

// Parser module types
declare module 'windmill-parser-wasm-ts' {
	export default function initTsParser(wasmUrl: string): Promise<void>
	export function parse_deno(code: string, mainOverride?: string): string
}

declare module 'windmill-parser-wasm-regex' {
	export default function initRegexParsers(wasmUrl: string): Promise<void>
	export function parse_sql(code: string): string
	export function parse_mysql(code: string): string
	export function parse_oracledb(code: string): string
	export function parse_bigquery(code: string): string
	export function parse_snowflake(code: string): string
	export function parse_graphql(code: string): string
	export function parse_mssql(code: string): string
	export function parse_db_resource(code: string): string | undefined
	export function parse_bash(code: string): string
	export function parse_powershell(code: string): string
}

declare module 'windmill-parser-wasm-py' {
	export default function initPythonParser(wasmUrl: string): Promise<void>
	export function parse_python(code: string, mainOverride?: string): string
}

declare module 'windmill-parser-wasm-go' {
	export default function initGoParser(wasmUrl: string): Promise<void>
	export function parse_go(code: string): string
}

declare module 'windmill-parser-wasm-php' {
	export default function initPhpParser(wasmUrl: string): Promise<void>
	export function parse_php(code: string): string
}

declare module 'windmill-parser-wasm-rust' {
	export default function initRustParser(wasmUrl: string): Promise<void>
	export function parse_rust(code: string): string
}

declare module 'windmill-parser-wasm-yaml' {
	export default function initYamlParser(wasmUrl: string): Promise<void>
	export function parse_ansible(code: string): string
}

declare module 'windmill-parser-wasm-csharp' {
	export default function initCSharpParser(wasmUrl: string): Promise<void>
	export function parse_csharp(code: string): string
}

declare module 'windmill-parser-wasm-nu' {
	export default function initNuParser(wasmUrl: string): Promise<void>
	export function parse_nu(code: string): string
}

declare module 'windmill-parser-wasm-java' {
	export default function initJavaParser(wasmUrl: string): Promise<void>
	export function parse_java(code: string): string
}

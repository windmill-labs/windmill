export const FIX_PROMPT = {
	system:
		'You fix the code shared by the user. Only output code. Wrap the code like that:\n```language\n{code}\n```\nExplain the error and the fix in the following format:\nexplanation: "Here\'s the explanation"\nAlso put the explanations in the code as comments.',
	prompts: {
		python3: {
			prompt:
				'Here\'s my python3 code: \n```python\n{code}\n```\nAdditional information: We have to export a "main" function and specify the parameter types but do not call it.\nYou have access to the following resource types, if you need them, you have to define the TypedDict exactly as specified (class name has to be IN LOWERCASE) and add them as parameters: {resourceTypes}\nOnly use the ones you need. If the TypedDict name conflicts with the imported object, rename the imported object NOT THE TYPE.\nI get the following error: {error}\nFix my code.',
			example_code: 'print("Hello world")',
			example_error:
				'{"error":{"name":"ExecutionErr","message":"error during execution of the script:\\\\nNo main function found"}}`',
			example_answer:
				'```python\ndef main():\n    print("Hello world")\n```\nexplanation: "The code should be wrapped it in a function called "main"."'
		},
		deno: {
			prompt:
				'Here\'s my TypeScript code in a deno running environment:\n```typescript\n{code}\n```\nAdditional information: We have to export a "main" function like this: "export async function main(...)" and specify the parameter types but do not call it.\nYou have access to the following resource types, if you need them, you have to define the type exactly as specified and add them as parameters: {resourceTypes}\nOnly use the ones you need. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE.\nI get the following error: {error}\nFix my code.',
			example_code: 'console.log("Hello world")',
			example_error:
				'{"error":{"name":"ExecutionErr","message":"error during execution of the script:\\\\nmain function was not findable (expected to find \'export function main(...)\'"}}`',
			example_answer:
				'```typescript\nexport async function main() {\n    console.log("Hello world")\n}\n```\nexplanation: "The code should be wrapped in an exported function called "main"."'
		},
		go: {
			prompt:
				'Here\'s my go code: \n```go\n{code}\n```\nAdditional information: We have to export a "main" function. Import the packages you need. The return type of the function has to be ({return_type}, error). The file package has to be "inner"\nI get the following error: {error}\nFix my code.',
			example_code:
				'package inner\n\nimport (\n  "fmt"\n)\n\nfunc main() (interface{}) {\n  fmt.Println("Hello World")\n  return nil\n}',
			example_error:
				'{"error":{"name":"ExecutionErr","message":"ExitCode: 2, last log lines:\\\\n# mymod/inner\\\\ninner/runner.go:7:12: not enough return values\\\\n\\\\thave (interface{})\\\\n\\\\twant (interface{}, error)"}}',
			example_answer:
				'```go\npackage inner\n\nimport (\n  "fmt"\n)\n\nfunc main() (interface{}, error) {\n  fmt.Println("Hello World")\n  return nil, nil\n}\n```\nexplanation: "The return type of the function should be ({return_type}, error) and therefore the function should return two parameters: the result and an error or nil."'
		},
		bash: {
			prompt:
				'Here\'s my bash code: \n```shell\n{code}\n```\nAdditional information: Do not include "#!/bin/bash". Arguments are always string and can only be obtained with "var1="$1"", "var2="$2"", etc... You do not need to check if the arguments are present.\nI get the following error: {error}\nFix my code.',
			example_code: '# shellcheck shell=bash\nmsg = "$1"\necho "Hello $msg"',
			example_error:
				'{"error":{"name":"ExecutionErr","message":"ExitCode: 127, last log lines:\\\\nmain.sh: line 4: msg: command not found"}}',
			example_answer:
				'```shell\n# shellcheck shell=bash\nmsg="$1"\necho "Hello $msg"\n```\nexplanation: "The variable assignment should be msg="$1"."'
		},
		postgresql: {
			prompt:
				"Here's my PostgreSQL code: \n```sql\n{code}\n```\nAdditional information: Arguments can be obtained directly in the statement with `$1::{type}`, `$2::{type}`, etc... Name the parameters by adding comments before the command like that: `-- $1 name1` or `-- $2 name = default` (one per row, do not include the type)\nI get the following error: {error}\nFix my code.",
			example_code: 'selec * from demo',
			example_error:
				'{"error":{"name":"ExecutionErr","message":"error during execution of the script:\\\\ndb error: ERROR: syntax error at or near \\\\"selec\\\\""}}',
			example_answer:
				'```sql\nselect * from demo\n```\nexplanation: "You\'re missing a "t" in "select"."'
		},
		mysql: {
			prompt:
				"Here's my MySQL code: \n```sql\n{code}bonjour\n```\nAdditional information: Arguments can be obtained directly in the statement with ?. Name the parameters by adding comments before the command like that: -- ? name1 ({type}) (one per row)\nI get the following error: {error}\nFix my code.",
			example_code: 'selec * from demo',
			example_error:
				'{"error":{"name":"ExecutionErr","message":"error during execution of the script:\\\\ndb error: ERROR: syntax error at or near \\\\"selec\\\\""}}',
			example_answer:
				'```sql\nselect * from demo\n``` \nexplanation: "You\'re missing a "t" in "select"."'
		},
		nativets: {
			prompt:
				'Here\'s my TypeScript code: \n```typescript\n{code}\n```\nAdditional information: We have to export a "main" function like this: "export async function main(...)" and specify the parameter types but do not call it.\nYou should use fetch and are not allowed to import any libraries.\nYou have access to the following resource types, if you need them, you have to define the type exactly as specified and add them as parameters: {resourceTypes}\nOnly use the ones you need. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE.\nI get the following error: {error}\nFix my code.',
			example_code:
				'await fetch("https://jsonplaceholder.typicode.com/todos/1", {\n  headers: { "Content-Type": "application/json" },\n});',
			example_error:
				'{"error":{"name":"ExecutionErr","message":"error during execution of the script:\\\\nmain function was not findable (expected to find \'export function main(...)\'"}}`',
			example_answer:
				'```typescript\nexport async function main() {\n  const result = await fetch("https://jsonplaceholder.typicode.com/todos/1", {\n    headers: { "Content-Type": "application/json" },\n  });\n  return result.json();\n}\n```\nexplanation: "The code should be wrapped in an exported function called "main"."'
		},
		bun: {
			prompt:
				'Here\'s my TypeScript code in a node.js running environment: \n```typescript\n{code}\n```\nAdditional information: We have to export a "main" function like this: "export async function main(...)" and specify the parameter types but do not call it.\nYou have access to the following resource types, if you need them, you have to define the type exactly as specified and add them as parameters: {resourceTypes}\nOnly use the ones you need. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE.\nI get the following error: {error}\nFix my code.',
			example_code: 'console.log("Hello world")',
			example_error:
				'{"error":{"name":"ExecutionErr","message":"error during execution of the script:\\\\nmain function was not findable (expected to find \'export function main(...)\'"}}`',
			example_answer:
				'```typescript\nexport async function main() {\n    console.log("Hello world")\n}\n```\nexplanation: "The code should be wrapped in an exported function called "main"."'
		}
	}
}

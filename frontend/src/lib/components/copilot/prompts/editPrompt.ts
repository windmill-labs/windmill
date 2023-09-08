export const EDIT_PROMPT = {
	system:
		'You write code as instructed by the user. Only output code. Wrap the code like that: \n```language\n{code}\n```\nPut explanations directly in the code as comments.',
	prompts: {
		python3: {
			prompt:
				'Here\'s my python3 code: \n```python\n{code}\n```\nAdditional information: We have to export a "main" function and specify the parameter types but do not call it.\nYou have access to the following resource types, if you need them, you have to define the TypedDict exactly as specified (class name has to be IN LOWERCASE) and add them as parameters: {resourceTypes}\nOnly use the ones you need. If the TypedDict name conflicts with the imported object, rename the imported object NOT THE TYPE.\nMy instructions: {description}',
			example_description: 'add comments',
			example_code:
				'import random\n\ndef main(name: str):\n    print("hello", name)\n    return random.randint(0, 100)',
			example_answer:
				'```python\nimport random\n\ndef main(name: str):\n    print("hello", name) # print hello name to the console\n    return random.randint(0, 100) # return a random number between 0 and 100\n```'
		},
		deno: {
			prompt:
				'Here\'s my TypeScript code in a deno running environment:\n```typescript\n{code}\n```\nAdditional information: We have to export a "main" function like this: "export async function main(...)" and specify the parameter types but do not call it.\nYou have access to the following resource types, if you need them, you have to define the type exactly as specified and add them as parameters: {resourceTypes}\nOnly use the ones you need. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE.\nMy instructions: {description}',
			example_description: 'add comments',
			example_code:
				'export async function main(name: string) {\n  console.log("hello", name)\n  return Math.floor(Math.random() * 100)\n}',
			example_answer:
				'```typescript\nexport async function main(name: string) {\n  console.log("hello", name) // print hello name to the console\n  return Math.floor(Math.random() * 100) // return a random number between 0 and 100\n}\n```'
		},
		go: {
			prompt:
				'Here\'s my go code: \n```go\n{code}\n```\nAdditional information: We have to export a "main" function. Import the packages you need. The return type of the function has to be ({return_type}, error). The file package has to be "inner"\nMy instructions: {description}',
			example_description: 'add comments',
			example_code:
				'package inner\n\nimport (\n  "fmt"\n  "math/rand"\n)\n\nfunc main(name string) (int, error) {\n  fmt.Println("hello", name)\n  return rand.Intn(100), nil\n}',
			example_answer:
				'```go\npackage inner\n\nimport (\n  "fmt"\n  "math/rand"\n)\n\nfunc main(name string) (int, error) {\n  fmt.Println("hello", name) // print hello name to the console\n  return rand.Intn(100), nil // return a random number between 0 and 100\n}\n```'
		},
		bash: {
			prompt:
				'Here\'s my bash code: \n```shell\n{code}\n```\nAdditional information: Do not include "#!/bin/bash". Arguments are always string and can only be obtained with "var1="$1"", "var2="$2"", etc... You do not need to check if the arguments are present.\nMy instructions: {description}',
			example_description: 'add comments',
			example_code: '# shellcheck shell=bash\nname="$1"\necho "hello $name"',
			example_answer:
				'```shell\n# shellcheck shell=bash\nname="$1" # get the name argument\necho "hello $name" # print hello name\n```'
		},
		postgresql: {
			prompt:
				"Here's my PostgreSQL code: \n```sql\n{code}\n```\nAdditional information: Arguments can be obtained directly in the statement with `$1::{type}`, `$2::{type}`, etc... Name the parameters by adding comments before the command like that: `-- $1 name1` or `-- $2 name = default` (one per row, do not include the type)\nMy instructions: {description}",
			example_description: 'also insert a product description',
			example_code:
				'-- $1 name\n-- $2 price\nINSERT INTO products (name, price) VALUES ($1::text, $2::numeric)',
			example_answer:
				'```sql\n-- $1 name\n-- $2 price\n-- $3 description\nINSERT INTO products (name, price, description) VALUES ($1::text, $2::numeric, $3::text)\n```'
		},
		mysql: {
			prompt:
				"Here's my MySQL code: \n```sql\n{code}\n```\nAdditional information: Arguments can be obtained directly in the statement with ?. Name the parameters by adding comments before the command like that: -- ? name1 ({type}) (one per row)\nMy instructions: {description}",
			example_description: 'also insert a product description',
			example_code:
				'-- ? name (text)\n-- ? price (float)\nINSERT INTO products (name, price) VALUES (?, ?)',
			example_answer:
				'```sql\n-- $1 name (text)\n-- $2 price (float)\n-- $3 description (text)\nINSERT INTO products (name, price, description) VALUES (?, ?, ?)\n```'
		},
		nativets: {
			prompt:
				'Here\'s my TypeScript code: \n```typescript\n{code}\n```\nAdditional information: We have to export a "main" function like this: "export async function main(...)" and specify the parameter types but do not call it.\nYou should use fetch and are not allowed to import any libraries.\nYou have access to the following resource types, if you need them, you have to define the type exactly as specified and add them as parameters: {resourceTypes}\nOnly use the ones you need. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE.\nMy instructions: {description}',
			example_description: 'add comments',
			example_code:
				'export async function main() {\n  const res = await fetch("https://jsonplaceholder.typicode.com/todos/1", {\n    headers: { "Content-Type": "application/json" },\n  });\n  return res.json();\n}',
			example_answer:
				'```typescript\nexport async function main() {\n  // fetch the data from jsonplaceholder\n  const res = await fetch("https://jsonplaceholder.typicode.com/todos/1", {\n    headers: { "Content-Type": "application/json" },\n  });\n  return res.json();\n}\n```'
		},
		bun: {
			prompt:
				'Here\'s my TypeScript code in a node.js running environment: \n```typescript\n{code}\n```\nAdditional information: We have to export a "main" function like this: "export async function main(...)" and specify the parameter types but do not call it.\nYou have access to the following resource types, if you need them, you have to define the type exactly as specified and add them as parameters: {resourceTypes}\nOnly use the ones you need. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE.\nMy instructions: {description}',
			example_description: 'add comments',
			example_code:
				'export async function main(name: string) {\n  console.log("hello", name)\n  return Math.floor(Math.random() * 100)\n}',
			example_answer:
				'```typescript\nexport async function main(name: string) {\n  console.log("hello", name) // print hello name to the console\n  return Math.floor(Math.random() * 100) // return a random number between 0 and 100\n}\n```'
		},
		frontend: {
			prompt:
				"Here's my client-side javascript code: \n```javascript\n{code}\n```\nAdditional information: You can access the context object with the ctx global variable. \nThe app state is a store that can be used to store data. You can access and update the state object with the state global variable like this: state.foo = 'bar'\nYou can use the goto function to navigate to a specific URL: goto(path: string, newTab?: boolean)\nUse the setTab function to manually set the tab of a Tab component: setTab(id: string, index: string)\nUse the recompute function to recompute a component: recompute(id: string)\nUse the getAgGrid function to get the ag-grid instance of a table: getAgGrid(id: string)\nThe setValue function is meant to set or force the value of a component: setValue(id: string, value: any).\nMy instructions: {description}",
			example_description: 'add comments',
			example_code: "var email = ctx.email;\n\nsetValue('my_field', email);",
			example_answer:
				"```javascript\n// Access the email from the context object\nvar email = ctx.email;\n\n// Use the setValue function to set the value of the input field with id 'my_field' to the email\nsetValue('my_field', email);\n```"
		}
	}
}

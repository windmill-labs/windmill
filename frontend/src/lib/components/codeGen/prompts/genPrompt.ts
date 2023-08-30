export const GEN_PROMPT = {
	system:
		'You write code as queried by the user. Only output code. Wrap the code like that: \n```language\n{code}\n```\nPut explanations directly in the code as comments.',
	prompts: {
		python3: {
			prompt:
				'Write a function in python called "main". The function should {description}. Specify the parameter types. Do not call the main function.\nYou have access to the following resource types, if you need them, you have to define the TypedDict exactly as specified (class name has to be IN LOWERCASE) and add them as parameters: {resourceTypes}\nOnly use the ones you need. If the TypedDict name conflicts with the imported object, rename the imported object NOT THE TYPE.',
			example_description:
				'say hello to the passed name and return a random number between 0 and 100',
			example_answer:
				'```python\nimport random\n\ndef main(name: str):\n    print("hello", name) # print hello name to the console\n    return random.randint(0, 100) # return a random number between 0 and 100\n```'
		},
		deno: {
			prompt:
				'Write a function in TypeScript called "main". The function should {description}. Specify the parameter types. You are in a Deno environment. You can import deno libraries or you can also import npm libraries like that: "import ... from "npm:{package}";". Export the "main" function like this: "export async function main(...)". Do not call the main function.\nYou have access to the following resource types, if you need them, you have to define the type exactly as specified and add them as parameters: {resourceTypes}\nOnly use the ones you need. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE.',
			example_description:
				'say hello to the passed name and return a random number between 0 and 100',
			example_answer:
				'```typescript\nexport async function main(name: string) {\n  console.log("hello", name) // print hello name to the console\n  return Math.floor(Math.random() * 100) // return a random number between 0 and 100\n}\n```'
		},
		go: {
			prompt:
				'Write a function in go called "main". The function should {description}. Import the packages you need. The return type of the function has to be ({return_type}, error). The file package has to be "inner".',
			example_description:
				'say hello to the passed name and return a random number between 0 and 100',
			example_answer:
				'```go\npackage inner\n\nimport (\n  "fmt"\n  "math/rand"\n)\n\nfunc main(name string) (int, error) {\n  fmt.Println("hello", name) // print hello name to the console\n  return rand.Intn(100), nil // return a random number between 0 and 100\n}\n```'
		},
		bash: {
			prompt:
				'Write bash code that should {description}. Do not include "#!/bin/bash". Arguments are always string and can only be obtained with "var1="$1"", "var2="$2"", etc... You do not need to check if the arguments are present.',
			example_description: 'say hello to the passed name',
			example_answer:
				'```shell\n# shellcheck shell=bash\nname="$1" # get the name argument\necho "hello $name" # print hello name\n```'
		},
		postgresql: {
			prompt:
				'Write SQL code for a PostgreSQL that should {description}. Arguments can be obtained directly in the statement with `$1::{type}`, `$2::{type}`, etc... Name the parameters by adding comments before the command like that: `-- $1 name1` or `-- $2 name = default` (one per row, do not include the type)',
			example_description: 'insert a name and a price into the products table',
			example_answer:
				'```sql\n-- $1 name\n-- $2 price\nINSERT INTO products (name, price) VALUES ($1::text, $2::numeric)\n```'
		},
		mysql: {
			prompt:
				'Write SQL code for MySQL that should {description}. Arguments can be obtained directly in the statement with ?. Name the parameters by adding comments before the command like that: -- ? name1 ({type}) (one per row)',
			example_description: 'insert a name and a price into the products table',
			example_answer:
				'```sql\n-- ? name (text)\n-- ? price (float)\nINSERT INTO products (name, price) VALUES (?, ?)\n```'
		},
		nativets: {
			prompt:
				'Write a function in TypeScript called "main". The function should {description}. Specify the parameter types. You should use fetch and are not allowed to import any libraries. Export the "main" function like this: "export async function main(...)". Do not call the main function.\nYou have access to the following resource types, if you need them, you have to define the type exactly as specified and add them as parameters: {resourceTypes}\nOnly use the ones you need. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE.',
			example_description: 'query sample data from jsonplaceholder',
			example_answer:
				'```typescript\nexport async function main() {\n  // fetch the data from jsonplaceholder\n  const res = await fetch("https://jsonplaceholder.typicode.com/todos/1", {\n    headers: { "Content-Type": "application/json" },\n  });\n  return res.json();\n}\n```'
		},
		bun: {
			prompt:
				'Write a function in TypeScript called "main". The function should {description}. Specify the parameter types. You are in a Node.js environment. You can import npm libraries. Export the "main" function like this: "export async function main(...)". Do not call the main function.\nYou have access to the following resource types, if you need them, you have to define the type exactly as specified and add them as parameters: {resourceTypes}\nOnly use the ones you need. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE.',
			example_description:
				'say hello to the passed name and return a random number between 0 and 100',
			example_answer:
				'```typescript\nexport async function main(name: string) {\n  console.log("hello", name) // print hello name to the console\n  return Math.floor(Math.random() * 100) // return a random number between 0 and 100\n}\n```'
		},
		frontend: {
			prompt:
				"Write client-side javascript code that should {description}. You have access to a few helpers:\nYou can access the context object with the ctx global variable. \nThe app state is a store that can be used to store data. You can access and update the state object with the state global variable like this: state.foo = 'bar'\nYou can use the goto function to navigate to a specific URL: goto(path: string, newTab?: boolean)\nUse the setTab function to manually set the tab of a Tab component: setTab(id: string, index: string)\nUse the recompute function to recompute a component: recompute(id: string)\nUse the getAgGrid function to get the ag-grid instance of a table: getAgGrid(id: string)\nThe setValue function is meant to set or force the value of a component: setValue(id: string, value: any).",
			example_description:
				"set the value of the input with id 'my_field' to the context variable email",
			example_answer:
				"```javascript\n// Access the email from the context object\nvar email = ctx.email;\n\n// Use the setValue function to set the value of the input field with id 'my_field' to the email\nsetValue('my_field', email);\n```"
		}
	}
}

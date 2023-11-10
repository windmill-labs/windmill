export const GEN_PROMPT = {
  "system": "You write code as instructed by the user. Only output code. Wrap the code in a code block. \nPut explanations directly in the code as comments.\n\nHere's how interactions have to look like:\nuser: {sample_question}\nassistant: ```language\n{code}\n```",
  "prompts": {
    "python3": {
      "prompt": "Write a function in python called \"main\". The function should {description}. Specify the parameter types. Do not call the main function.\nYou can take as parameters resources which are dictionaries containing credentials or configuration information. Name the resource parameters like this: \"{resource_type}_resource\".\nThe following resource types are available:\n<resourceTypes>\n{resourceTypes}\n</resourceTypes>\nOnly define the type for resources that are actually needed to achieve the function purpose. The resource type name has to be exactly as specified (has to be IN LOWERCASE). If the type name conflicts with the imported object, rename the imported object NOT THE TYPE."
    },
    "deno": {
      "prompt": "Write a function in TypeScript called \"main\". The function should {description}. Specify the parameter types. You are in a Deno environment. You can import deno libraries or you can also import npm libraries like that: \"import ... from \"npm:{package}\";\". Export the \"main\" function like this: \"export async function main(...)\". Do not call the main function.\nIf needed, the standard fetch method is available globally, do not import it.\nYou can take as parameters resources which are dictionaries containing credentials or configuration information. Name the resource parameters like this: \"{resource_type}Resource\".\nThe following resource types are available:\n<resourceTypes>\n{resourceTypes}\n</resourceTypes>\nOnly define the type for resources that are actually needed to achieve the function purpose. The resource type name has to be exactly as specified. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE."
    },
    "go": {
      "prompt": "Write a function in go called \"main\". The function should {description}. Import the packages you need. The return type of the function has to be ({return_type}, error). The file package has to be \"inner\"."
    },
    "bash": {
      "prompt": "Write bash code that should {description}. Do not include \"#!/bin/bash\". Arguments are always string and can only be obtained with \"var1=\"$1\"\", \"var2=\"$2\"\", etc... You do not need to check if the arguments are present."
    },
    "postgresql": {
      "prompt": "Write SQL code for a PostgreSQL that should {description}. Arguments can be obtained directly in the statement with `$1::{type}`, `$2::{type}`, etc... Name the parameters (without specifying the type) by adding comments before the statement like that: `-- $1 name1` or `-- $2 name = default` (one per row)"
    },
    "mysql": {
      "prompt": "Write SQL code for MySQL that should {description}. Arguments can be obtained directly in the statement with ?. Name the parameters by adding comments before the statement like that: `-- ? name1 ({type})` or `-- ? name2 ({type}) = default` (one per row)"
    },
    "bigquery": {
      "prompt": "Write SQL code for BigQuery that should {description}. You can define arguments by adding comments before the statement like that: `-- @name1 ({type})` or `-- @name2 ({type}) = default` (one per row). They can then be obtained directly in the statement with `@name1`, `@name2`, etc...."
    },
    "snowflake": {
      "prompt": "Write SQL code for snowflake that should {description}. Arguments can be obtained directly in the statement with ?. Name the parameters by adding comments before the statement like that: `-- ? name1 ({type})` or `-- ? name2 ({type}) = default` (one per row)"
    },
    "mssql": {
      "prompt": "Write SQL code for Microsoft SQL Server that should {description}. Arguments can be obtained directly in the statement with @p1, @p2, etc.. Name the parameters by adding comments before the statement like that: `-- @p1 name1 ({type})` or `-- @p2 name2 ({type}) = default` (one per row)"
    },
    "graphql": {
      "prompt": "Write a GraphQL query that should {description}. Add the needed arguments as query parameters."
    },
    "powershell": {
      "prompt": "Write powershell code that should {description}. Arguments can be obtained by calling the param function on the first line like that: `param($ParamName1, $ParamName2 = \"default value\", [{type}]$ParamName3, ...)`"
    },
    "nativets": {
      "prompt": "Write a function in TypeScript called \"main\". The function should {description}. Specify the parameter types. You should use fetch and are not allowed to import any libraries. Export the \"main\" function like this: \"export async function main(...)\". Do not call the main function.\nYou can take as parameters resources which are dictionaries containing credentials or configuration information. Name the resource parameters like this: \"{resource_type}Resource\".\nThe following resource types are available:\n<resourceTypes>\n{resourceTypes}\n</resourceTypes>\nOnly define the type for resources that are actually needed to achieve the function purpose. The resource type name has to be exactly as specified. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE."
    },
    "bun": {
      "prompt": "Write a function in TypeScript called \"main\". The function should {description}. Specify the parameter types. You can import npm libraries. Export the \"main\" function like this: \"export async function main(...)\". Do not call the main function.\nIf needed, the standard fetch method is available globally, do not import it.\nYou can take as parameters resources which are dictionaries containing credentials or configuration information. Name the resource parameters like this: \"{resource_type}Resource\".\nThe following resource types are available:\n<resourceTypes>\n{resourceTypes}\n</resourceTypes>\nOnly define the type for resources that are actually needed to achieve the function purpose. The resource type name has to be exactly as specified. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE."
    },
    "frontend": {
      "prompt": "Write client-side javascript code that should {description}. You have access to a few helpers:\nYou can access the context object with the ctx global variable. \nThe app state is a store that can be used to store data. You can access and update the state object with the state global variable like this: state.foo = 'bar'\nYou can use the goto function to navigate to a specific URL: goto(path: string, newTab?: boolean)\nUse the setTab function to manually set the tab of a Tab component: setTab(id: string, index: string)\nUse the recompute function to recompute a component: recompute(id: string)\nUse the getAgGrid function to get the ag-grid instance of a table: getAgGrid(id: string)\nThe setValue function is meant to set or force the value of a component: setValue(id: string, value: any)."
    }
  }
};
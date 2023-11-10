export const FIX_PROMPT = {
  "system": "You fix the code shared by the user. Only output code. Wrap the code in a code block. \nExplain the error and the fix after generating the code inside an <explanation> tag.\nAlso put explanations directly in the code as comments.\n\nHere's how interactions have to look like:\nuser: {sample_question}\nassistant: ```language\n{code}\n```\n<explanation>{explanation}</explanation>",
  "prompts": {
    "python3": {
      "prompt": "Here's my python3 code: \n```python\n{code}\n```\n<contextual_information>\nWe have to export a \"main\" function and specify the parameter types but do not call it.\nYou can take as parameters resources which are dictionaries containing credentials or configuration information. Name the resource parameters like this: \"{resource_type}_resource\". \nThe following resource types are available:\n<resourceTypes>\n{resourceTypes}\n</resourceTypes>\nOnly define the type for resources that are actually needed to achieve the function purpose. The resource type name has to be exactly as specified (has to be IN LOWERCASE). If the type name conflicts with the imported object, rename the imported object NOT THE TYPE.\n</contextual_information>\nI get the following error: {error}\nFix my code."
    },
    "deno": {
      "prompt": "Here's my TypeScript code in a deno running environment:\n```typescript\n{code}\n```\n<contextual_information>\nWe have to export a \"main\" function like this: \"export async function main(...)\" and specify the parameter types but do not call it.\nIf needed, the standard fetch method is available globally, do not import it.\nYou can take as parameters resources which are dictionaries containing credentials or configuration information. Name the resource parameters like this: \"{resource_type}Resource\".\nThe following resource types are available:\n<resourceTypes>\n{resourceTypes}\n</resourceTypes>\nOnly define the type for resources that are actually needed to achieve the function purpose. The resource type name has to be exactly as specified. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE.\n</contextual_information>\nI get the following error: {error}\nFix my code."
    },
    "go": {
      "prompt": "Here's my go code: \n```go\n{code}\n```\n<contextual_information>\nWe have to export a \"main\" function. Import the packages you need. The return type of the function has to be ({return_type}, error). The file package has to be \"inner\"\n</contextual_information>\nI get the following error: {error}\nFix my code."
    },
    "bash": {
      "prompt": "Here's my bash code: \n```shell\n{code}\n```\n<contextual_information>\nDo not include \"#!/bin/bash\". Arguments are always string and can only be obtained with \"var1=\"$1\"\", \"var2=\"$2\"\", etc... You do not need to check if the arguments are present.\n</contextual_information>\nI get the following error: {error}\nFix my code."
    },
    "postgresql": {
      "prompt": "Here's my PostgreSQL code: \n```sql\n{code}\n```\n<contextual_information>\nArguments can be obtained directly in the statement with `$1::{type}`, `$2::{type}`, etc... Name the parameters (without specifying the type) by adding comments before the statement like that: `-- $1 name1` or `-- $2 name = default` (one per row)\n</contextual_information>\nI get the following error: {error}\nFix my code."
    },
    "mysql": {
      "prompt": "Here's my MySQL code: \n```sql\n{code}\n```\n<contextual_information>\nArguments can be obtained directly in the statement with ?. Name the parameters by adding comments before the statement like that: `-- ? name1 ({type})` or `-- ? name2 ({type}) = default` (one per row)\n</contextual_information>\nI get the following error: {error}\nFix my code."
    },
    "bigquery": {
      "prompt": "Here's my BigQuery code: \n```sql\n{code}\n```\n<contextual_information>\nYou can define arguments by adding comments before the statement like that: `-- @name1 ({type})` or `-- @name2 ({type}) = default` (one per row). They can then be obtained directly in the statement with `@name1`, `@name2`, etc....\n</contextual_information>\nI get the following error: {error}\nFix my code."
    },
    "snowflake": {
      "prompt": "Here's my snowflake code: \n```sql\n{code}\n```\n<contextual_information>\nArguments can be obtained directly in the statement with ?. Name the parameters by adding comments before the statement like that: `-- ? name1 ({type})` or `-- ? name2 ({type}) = default` (one per row)\n</contextual_information>\nI get the following error: {error}\nFix my code."
    },
    "mssql": {
      "prompt": "Here's my Microsoft SQL Server code: \n```sql\n{code}\n```\n<contextual_information>\nArguments can be obtained directly in the statement with @p1, @p2, etc.. Name the parameters by adding comments before the statement like that: `-- @p1 name1 ({type})` or `-- @p2 name2 ({type}) = default` (one per row)\n</contextual_information>\nI get the following error: {error}\nFix my code."
    },
    "graphql": {
      "prompt": "Here's my graphql code: \n```graphql\n{code}\n```\n<contextual_information>\nAdd the needed arguments as query parameters.\n</contextual_information>\nI get the following error: {error}\nFix my code."
    },
    "powershell": {
      "prompt": "Here's my powershell code: \n```powershell\n{code}\n```\n<contextual_information>\nArguments can be obtained by calling the param function on the first line like that: `param($ParamName1, $ParamName2 = \"default value\", [{type}]$ParamName3, ...)`\n</contextual_information>\nI get the following error: {error}\nFix my code."
    },
    "nativets": {
      "prompt": "Here's my TypeScript code: \n```typescript\n{code}\n```\n<contextual_information>\nWe have to export a \"main\" function like this: \"export async function main(...)\" and specify the parameter types but do not call it.\nYou can take as parameters resources which are dictionaries containing credentials or configuration information. Name the resource parameters like this: \"{resource_type}Resource\". \nThe following resource types are available:\n<resourceTypes>\n{resourceTypes}\n</resourceTypes>\nOnly define the type for resources that are actually needed to achieve the function purpose. The resource type name has to be exactly as specified. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE.\n</contextual_information>\nI get the following error: {error}\nFix my code."
    },
    "bun": {
      "prompt": "Here's my TypeScript code: \n```typescript\n{code}\n```\n<contextual_information>\nWe have to export a \"main\" function like this: \"export async function main(...)\" and specify the parameter types but do not call it.\nIf needed, the standard fetch method is available globally, do not import it.\nYou can take as parameters resources which are dictionaries containing credentials or configuration information. Name the resource parameters like this: \"{resource_type}Resource\".\nThe following resource types are available:\n<resourceTypes>\n{resourceTypes}\n</resourceTypes>\nOnly define the type for resources that are actually needed to achieve the function purpose. The resource type name has to be exactly as specified. If the type name conflicts with the imported object, rename the imported object NOT THE TYPE.\n</contextual_information>\nI get the following error: {error}\nFix my code."
    }
  }
};
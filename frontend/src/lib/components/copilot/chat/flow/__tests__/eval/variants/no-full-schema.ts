import type { ChatCompletionFunctionTool } from 'openai/resources/chat/completions.mjs'
import type { VariantConfig } from '../evalVariants'
import { flowTools } from '../../../core'
import type { Tool } from '../../../../shared'
import type { FlowAIChatHelpers } from '../../../core'
import { findModuleInFlow } from '../../../utils'
import type { FlowModule } from '$lib/gen'

/**
 * Simplified InputTransform schema (inline, no $ref)
 */
const inputTransformSchema = {
	oneOf: [
		{
			type: 'object',
			description:
				"Static value passed directly. For resources, use format '$res:path/to/resource'",
			properties: {
				value: { description: 'The static value' },
				type: { type: 'string', enum: ['static'] }
			},
			required: ['type']
		},
		{
			type: 'object',
			description:
				"JavaScript expression evaluated at runtime. Use 'results.step_id' or 'flow_input.property'. Inside loops, use 'flow_input.iter.value'",
			properties: {
				expr: { type: 'string', description: 'JavaScript expression returning the value' },
				type: { type: 'string', enum: ['javascript'] }
			},
			required: ['expr', 'type']
		}
	]
}

/**
 * Simplified FlowModuleValue schema without circular references.
 * Container types (forloopflow, whileloopflow, branchone, branchall, aiagent)
 * have their nested modules/tools arrays marked as must-be-empty.
 */
const simplifiedFlowModuleValueSchema = {
	description:
		'The module implementation. For containers (loops, branches), modules array must be empty - use add_module with insideId to add steps inside.',
	oneOf: [
		// RawScript - no nested modules, keep full schema
		{
			type: 'object',
			description:
				"Inline script with code. Use 'bun' as default language. Script receives arguments from input_transforms",
			properties: {
				type: { type: 'string', enum: ['rawscript'] },
				content: {
					type: 'string',
					description: "Script source code. Should export a 'main' function"
				},
				language: {
					type: 'string',
					enum: [
						'deno',
						'bun',
						'python3',
						'go',
						'bash',
						'powershell',
						'postgresql',
						'mysql',
						'bigquery',
						'snowflake',
						'mssql',
						'oracledb',
						'graphql',
						'nativets',
						'php'
					],
					description: 'Programming language'
				},
				input_transforms: {
					type: 'object',
					description: 'Map of parameter names to values (static or JavaScript expressions)',
					additionalProperties: inputTransformSchema
				},
				path: { type: 'string', description: 'Optional path for saving this script' },
				lock: { type: 'string', description: 'Lock file content for dependencies' },
				tag: { type: 'string', description: 'Worker group tag for execution routing' },
				concurrent_limit: { type: 'number' },
				concurrency_time_window_s: { type: 'number' },
				custom_concurrency_key: { type: 'string' },
				is_trigger: { type: 'boolean' }
			},
			required: ['type', 'content', 'language', 'input_transforms']
		},
		// PathScript - reference to existing script
		{
			type: 'object',
			description: 'Reference to an existing script by path',
			properties: {
				type: { type: 'string', enum: ['script'] },
				path: { type: 'string', description: "Path to script (e.g., 'f/scripts/send_email')" },
				hash: { type: 'string', description: 'Optional specific version hash' },
				input_transforms: {
					type: 'object',
					description: 'Map of parameter names to values',
					additionalProperties: inputTransformSchema
				},
				tag_override: { type: 'string' },
				is_trigger: { type: 'boolean' }
			},
			required: ['type', 'path', 'input_transforms']
		},
		// PathFlow - reference to existing flow
		{
			type: 'object',
			description: 'Reference to an existing flow as a subflow',
			properties: {
				type: { type: 'string', enum: ['flow'] },
				path: { type: 'string', description: "Path to flow (e.g., 'f/flows/process_user')" },
				input_transforms: {
					type: 'object',
					description: 'Map of parameter names to values',
					additionalProperties: inputTransformSchema
				}
			},
			required: ['type', 'path', 'input_transforms']
		},
		// ForloopFlow - modules MUST be empty
		{
			type: 'object',
			description:
				"For loop over an iterator. IMPORTANT: 'modules' must be an empty array []. Use add_module with insideId and branchPath='modules' to add steps inside.",
			properties: {
				type: { type: 'string', enum: ['forloopflow'] },
				modules: {
					type: 'array',
					items: {},
					description:
						"MUST be empty []. Use add_module({ insideId: 'loop_id', branchPath: 'modules', value: {...} }) to add steps"
				},
				iterator: {
					...inputTransformSchema,
					description:
						"JavaScript expression returning array to iterate. Use 'flow_input.iter.value' inside loop to access current item"
				},
				skip_failures: {
					type: 'boolean',
					description: "If true, iteration failures don't stop the loop"
				},
				parallel: {
					type: 'boolean',
					description: 'If true, iterations run concurrently'
				},
				parallelism: {
					...inputTransformSchema,
					description: 'Max concurrent iterations when parallel=true'
				}
			},
			required: ['type', 'modules', 'iterator', 'skip_failures']
		},
		// WhileloopFlow - modules MUST be empty
		{
			type: 'object',
			description:
				"While loop that repeats until stop_after_if triggers. IMPORTANT: 'modules' must be an empty array []. Use add_module to add steps inside.",
			properties: {
				type: { type: 'string', enum: ['whileloopflow'] },
				modules: {
					type: 'array',
					items: {},
					description:
						"MUST be empty []. Use add_module({ insideId: 'loop_id', branchPath: 'modules', value: {...} }) to add steps"
				},
				skip_failures: {
					type: 'boolean',
					description: "If true, iteration failures don't stop the loop"
				},
				parallel: { type: 'boolean' },
				parallelism: inputTransformSchema
			},
			required: ['type', 'modules', 'skip_failures']
		},
		// BranchOne - branches and default MUST be empty
		{
			type: 'object',
			description:
				"Conditional branching (first match wins). IMPORTANT: Create with empty 'branches' array [], then use add_module with branchPath=null to add branches, and branchPath='branches.N' to add modules inside branches.",
			properties: {
				type: { type: 'string', enum: ['branchone'] },
				branches: {
					type: 'array',
					items: {},
					description:
						"MUST be empty []. Use add_module({ insideId: 'branch_id', branchPath: null, value: { summary, expr, modules: [] } }) to add branches"
				},
				default: {
					type: 'array',
					items: {},
					description:
						"MUST be empty []. Use add_module({ insideId: 'branch_id', branchPath: 'default', value: {...} }) to add steps to default branch"
				}
			},
			required: ['type', 'branches', 'default']
		},
		// BranchAll - branches MUST be empty
		{
			type: 'object',
			description:
				"Parallel branching (all branches execute). IMPORTANT: Create with empty 'branches' array [], then use add_module with branchPath=null to add branches.",
			properties: {
				type: { type: 'string', enum: ['branchall'] },
				branches: {
					type: 'array',
					items: {},
					description:
						"MUST be empty []. Use add_module({ insideId: 'branch_id', branchPath: null, value: { summary, skip_failure, modules: [] } }) to add branches"
				},
				parallel: {
					type: 'boolean',
					description: 'If true, all branches execute concurrently'
				}
			},
			required: ['type', 'branches']
		},
		// Identity - pass-through
		{
			type: 'object',
			description: 'Pass-through module that returns input unchanged',
			properties: {
				type: { type: 'string', enum: ['identity'] },
				flow: { type: 'boolean' }
			},
			required: ['type']
		},
		// AiAgent - tools MUST be empty
		{
			type: 'object',
			description:
				"AI agent step. IMPORTANT: 'tools' must be an empty array []. Use add_module with branchPath='tools' to add tools.",
			properties: {
				type: { type: 'string', enum: ['aiagent'] },
				input_transforms: {
					type: 'object',
					description: 'Agent input parameters',
					properties: {
						provider: inputTransformSchema,
						output_type: inputTransformSchema,
						user_message: inputTransformSchema,
						system_prompt: inputTransformSchema,
						streaming: inputTransformSchema,
						messages_context_length: inputTransformSchema,
						output_schema: inputTransformSchema,
						user_images: inputTransformSchema,
						max_completion_tokens: inputTransformSchema,
						temperature: inputTransformSchema
					},
					required: ['provider', 'user_message', 'output_type']
				},
				tools: {
					type: 'array',
					items: {},
					description:
						"MUST be empty []. Use add_module({ insideId: 'agent_id', branchPath: 'tools', value: {...} }) to add tools"
				},
				parallel: { type: 'boolean' }
			},
			required: ['type', 'tools', 'input_transforms']
		}
	]
}

/**
 * Simplified FlowModule schema without circular references in nested modules
 */
const simplifiedFlowModuleSchema = {
	type: 'object',
	description: 'A single step in a flow',
	properties: {
		id: {
			type: 'string',
			description:
				"Unique identifier. Used to reference results via 'results.step_id'. Must be alphanumeric with underscores/hyphens"
		},
		value: simplifiedFlowModuleValueSchema,
		summary: { type: 'string', description: 'Short description of what this step does' },
		stop_after_if: {
			type: 'object',
			description: 'Early termination condition evaluated after step completes',
			properties: {
				skip_if_stopped: { type: 'boolean' },
				expr: { type: 'string', description: "Expression using 'result' or 'flow_input'" },
				error_message: { type: 'string' }
			},
			required: ['expr']
		},
		stop_after_all_iters_if: {
			type: 'object',
			description: 'For loops - condition evaluated after all iterations',
			properties: {
				skip_if_stopped: { type: 'boolean' },
				expr: { type: 'string' },
				error_message: { type: 'string' }
			},
			required: ['expr']
		},
		skip_if: {
			type: 'object',
			properties: {
				expr: { type: 'string', description: 'Expression returning true to skip this step' }
			},
			required: ['expr']
		},
		sleep: {
			...inputTransformSchema,
			description: 'Delay before executing (seconds)'
		},
		cache_ttl: { type: 'number', description: 'Cache duration in seconds' },
		timeout: {
			...inputTransformSchema,
			description: 'Max execution time in seconds'
		},
		delete_after_use: { type: 'boolean' },
		mock: {
			type: 'object',
			properties: {
				enabled: { type: 'boolean' },
				return_value: {}
			}
		},
		suspend: {
			type: 'object',
			description: 'Approval/resume configuration',
			properties: {
				required_events: { type: 'integer' },
				timeout: { type: 'integer' },
				resume_form: {
					type: 'object',
					properties: { schema: { type: 'object' } }
				},
				user_auth_required: { type: 'boolean' },
				user_groups_required: inputTransformSchema,
				self_approval_disabled: { type: 'boolean' },
				hide_cancel: { type: 'boolean' },
				continue_on_disapprove_timeout: { type: 'boolean' }
			}
		},
		priority: { type: 'number' },
		continue_on_error: { type: 'boolean' },
		retry: {
			type: 'object',
			properties: {
				constant: {
					type: 'object',
					properties: {
						attempts: { type: 'integer' },
						seconds: { type: 'integer' }
					}
				},
				exponential: {
					type: 'object',
					properties: {
						attempts: { type: 'integer' },
						multiplier: { type: 'integer' },
						seconds: { type: 'integer', minimum: 1 },
						random_factor: { type: 'integer', minimum: 0, maximum: 100 }
					}
				},
				retry_if: {
					type: 'object',
					properties: {
						expr: { type: 'string' }
					},
					required: ['expr']
				}
			}
		}
	},
	required: ['value', 'id']
}

/**
 * Custom add_module tool definition with simplified schema (no circular refs)
 */
const noSchemaAddModuleToolDef: ChatCompletionFunctionTool = {
	type: 'function',
	function: {
		strict: false,
		name: 'add_module',
		description:
			"Add a new module to the flow. For containers (loops, branches, agents), add with EMPTY modules array, then use additional add_module calls to add steps inside. Reserved IDs: 'failure', 'preprocessor', 'Input'.",
		parameters: {
			type: 'object',
			properties: {
				afterId: {
					type: ['string', 'null'],
					description: 'ID of module to insert after. Use null to insert at beginning.'
				},
				insideId: {
					type: ['string', 'null'],
					description:
						'ID of container module (branch/loop/agent) to insert into. Use with branchPath.'
				},
				branchPath: {
					type: ['string', 'null'],
					description:
						"Path inside container: 'modules' (loops), 'branches.0'/'branches.1'/etc (specific branch), 'default' (branchone default), 'tools' (aiagent). Use null with insideId to add NEW branch to branchall/branchone."
				},
				value: simplifiedFlowModuleSchema
			},
			required: ['value']
		}
	}
}

/**
 * Custom modify_module tool definition with simplified schema (no circular refs)
 */
const noSchemaModifyModuleToolDef: ChatCompletionFunctionTool = {
	type: 'function',
	function: {
		strict: false,
		name: 'modify_module',
		description:
			"Modify an existing module (full replacement). Use for changing configuration, transforms, or conditions. NOT for adding/removing nested modules - use add_module/remove_module. Reserved IDs: 'failure', 'preprocessor', 'Input'.",
		parameters: {
			type: 'object',
			properties: {
				id: {
					type: 'string',
					description: 'ID of the module to modify'
				},
				value: simplifiedFlowModuleSchema
			},
			required: ['id', 'value']
		}
	}
}

/**
 * Dedicated add_branch tool for adding branches to branchone/branchall containers.
 * This makes it clearer how to add branches without nested modules.
 */
const addBranchToolDef: ChatCompletionFunctionTool = {
	type: 'function',
	function: {
		strict: false,
		name: 'add_branch',
		description:
			'Add a new branch to a branchone or branchall container. The branch will have an empty modules array - use add_module with insideId and branchPath to add steps inside the branch.',
		parameters: {
			type: 'object',
			properties: {
				containerId: {
					type: 'string',
					description: 'ID of the branchone or branchall container to add a branch to'
				},
				summary: {
					type: 'string',
					description:
						'Short description of the branch (e.g., "Handle admin users", "Process errors")'
				},
				expr: {
					type: 'string',
					description:
						"JavaScript expression for branchone only. Return true to execute this branch. Can use 'results.step_id' or 'flow_input'. Example: \"results.check_role === 'admin'\""
				},
				skip_failure: {
					type: 'boolean',
					description:
						'For branchall only. If true, failure in this branch does not fail the entire flow. Default: false'
				}
			},
			required: ['containerId']
		}
	}
}

/**
 * Implementation for add_branch tool.
 * Adds a new branch with empty modules array to a branchone or branchall container.
 */
async function addBranchImpl({
	helpers,
	args
}: {
	args: { containerId: string; summary?: string; expr?: string; skip_failure?: boolean }
	helpers: FlowAIChatHelpers
}): Promise<string> {
	const { containerId, summary = '', expr, skip_failure = false } = args

	const flow = helpers.getFlowAndSelectedId().flow
	const container = findModuleInFlow(flow.value.modules, containerId)

	if (!container) {
		return `Error: Container with ID '${containerId}' not found in flow`
	}

	if (container.value.type !== 'branchone' && container.value.type !== 'branchall') {
		return `Error: Module '${containerId}' is not a branchone or branchall container (type: ${container.value.type})`
	}

	// Add branch to the container
	if (container.value.type === 'branchone') {
		const newBranch = {
			summary: summary,
			expr: expr || 'false',
			modules: [] as FlowModule[]
		}
		container.value.branches = [...(container.value.branches || []), newBranch]
		const branchIndex = container.value.branches.length - 1
		helpers.setFlowJson(JSON.stringify(flow))
		return `Added branch ${branchIndex} to branchone '${containerId}' with expr: "${newBranch.expr}". Use add_module({ insideId: "${containerId}", branchPath: "branches.${branchIndex}", value: {...} }) to add modules inside this branch.`
	} else {
		// branchall
		const newBranch = {
			summary: summary,
			skip_failure: skip_failure,
			modules: [] as FlowModule[]
		}
		container.value.branches = [...(container.value.branches || []), newBranch]
		const branchIndex = container.value.branches.length - 1
		helpers.setFlowJson(JSON.stringify(flow))
		return `Added branch ${branchIndex} to branchall '${containerId}'. Use add_module({ insideId: "${containerId}", branchPath: "branches.${branchIndex}", value: {...} }) to add modules inside this branch.`
	}
}

/**
 * Additional system prompt content for container module creation
 */
const CONTAINER_MODULE_INSTRUCTIONS = `

## Creating Container Modules (Loops, Branches, AI Agents)

IMPORTANT: When creating container modules, you MUST use a multi-step process:

1. **First**: Add the container module with an EMPTY modules/branches/tools array
2. **For branches**: Use the \`add_branch\` tool to add branches to branchone/branchall
3. **Then**: Use separate \`add_module\` calls to add modules inside the container

### For Loops (forloopflow)
\`\`\`javascript
// Step 1: Create the loop container (modules MUST be empty [])
add_module({ afterId: "previous_step", value: {
  id: "my_loop",
  value: { type: "forloopflow", modules: [], iterator: { type: "javascript", expr: "results.step_a" }, skip_failures: false }
}})

// Step 2: Add modules inside the loop
add_module({ insideId: "my_loop", branchPath: "modules", value: { id: "step_in_loop", value: { type: "rawscript", ... } }})
\`\`\`

### While Loops (whileloopflow)
\`\`\`javascript
// Step 1: Create with empty modules
add_module({ afterId: "previous_step", value: {
  id: "my_while",
  value: { type: "whileloopflow", modules: [], skip_failures: false }
}})

// Step 2: Add modules inside (use stop_after_if to control loop termination)
add_module({ insideId: "my_while", branchPath: "modules", value: {
  id: "check_condition",
  stop_after_if: { expr: "result.done === true" },
  value: { type: "rawscript", ... }
}})
\`\`\`

### Conditional Branches (branchone)
\`\`\`javascript
// Step 1: Create branch container with empty arrays
add_module({ afterId: "previous_step", value: {
  id: "my_branch",
  value: { type: "branchone", branches: [], default: [] }
}})

// Step 2: Use add_branch to add conditional branches
add_branch({ containerId: "my_branch", summary: "Condition 1", expr: "results.step_a > 10" })
add_branch({ containerId: "my_branch", summary: "Condition 2", expr: "results.step_a < 0" })

// Step 3: Add modules inside each branch (branches.0, branches.1, etc.)
add_module({ insideId: "my_branch", branchPath: "branches.0", value: { id: "step_if_positive", value: {...} }})
add_module({ insideId: "my_branch", branchPath: "branches.1", value: { id: "step_if_negative", value: {...} }})

// Step 4: Add modules to default branch (executed if no conditions match)
add_module({ insideId: "my_branch", branchPath: "default", value: { id: "step_default", value: {...} }})
\`\`\`

### Parallel Branches (branchall)
\`\`\`javascript
// Step 1: Create with empty branches
add_module({ afterId: "previous_step", value: {
  id: "parallel_tasks",
  value: { type: "branchall", branches: [], parallel: true }
}})

// Step 2: Use add_branch to add parallel branches
add_branch({ containerId: "parallel_tasks", summary: "Branch A" })
add_branch({ containerId: "parallel_tasks", summary: "Branch B", skip_failure: true })

// Step 3: Add modules to each branch (branches.0, branches.1, etc.)
add_module({ insideId: "parallel_tasks", branchPath: "branches.0", value: { id: "task_a", value: {...} }})
add_module({ insideId: "parallel_tasks", branchPath: "branches.1", value: { id: "task_b", value: {...} }})
\`\`\`

### AI Agents (aiagent)
\`\`\`javascript
// Step 1: Create agent with empty tools
add_module({ afterId: "previous_step", value: {
  id: "my_agent",
  value: {
    type: "aiagent",
    tools: [],
    input_transforms: {
      provider: { type: "static", value: "openai" },
      user_message: { type: "javascript", expr: "flow_input.question" },
      output_type: { type: "static", value: "text" }
    }
  }
}})

// Step 2: Add tools to the agent
add_module({ insideId: "my_agent", branchPath: "tools", value: {
  id: "search_tool",
  summary: "search_database",
  value: { tool_type: "flowmodule", type: "rawscript", language: "bun", content: "...", input_transforms: {} }
}})
\`\`\`
`

/**
 * Full system prompt for the no-full-schema variant
 */
const NO_FULL_SCHEMA_SYSTEM_PROMPT = `You are a helpful assistant that creates and edits workflows on the Windmill platform.

## IMPORTANT RULES

**Reserved IDs - Do NOT use these in add_module, modify_module, or remove_module:**
- \`failure\` - Reserved for failure handler module
- \`preprocessor\` - Reserved for preprocessor module
- \`Input\` - Reserved for flow input reference

## Tool Selection Guide

**Flow Modification:**
- **Add a new module** → \`add_module\`
- **Remove a module** → \`remove_module\`
- **Add a new branch to branchall/branchone** → \`add_branch\` (NOT add_module)
- **Remove a branch from branchall/branchone** → \`remove_branch\`
- **Change module code only** → \`set_module_code\`
- **Change module config/transforms/conditions** → \`modify_module\`
- **Update flow input parameters** → \`set_flow_schema\`

**Code & Scripts:**
- **View existing inline script code** → \`inspect_inline_script\`
- **Get language-specific coding instructions** → \`get_instructions_for_code_generation\` (call BEFORE writing code)
- **Find workspace scripts** → \`search_scripts\`
- **Find Windmill Hub scripts** → \`search_hub_scripts\`

**Testing:**
- **Test entire flow** → \`test_run_flow\`
- **Test single step** → \`test_run_step\`

**Resources & Schema:**
- **Search resource types** → \`resource_type\`
- **Get database schema** → \`get_db_schema\`

## Common Mistakes to Avoid

- **Don't use \`modify_module\` to add/remove nested modules** - Use \`add_module\`/\`remove_module\` instead
- **Don't forget \`input_transforms\`** - Rawscript parameters won't receive values without them
- **Don't use spaces in module IDs** - Use underscores (e.g., \`fetch_data\` not \`fetch data\`)
- **Don't reference future steps** - \`results.step_id\` only works for steps that execute before the current one
- **Don't create duplicate IDs** - Each module ID must be unique in the flow. Always generate fresh, unique IDs for new modules. Never reuse IDs from existing or previously removed modules
- **Don't provide nested modules directly** - Always create containers with empty arrays, then add modules via separate add_module calls
${CONTAINER_MODULE_INSTRUCTIONS}
## User Instructions

Follow the user instructions carefully.
At the end of your changes, explain precisely what you did and what the flow does now.
ALWAYS test your modifications. You have access to the \`test_run_flow\` and \`test_run_step\` tools to test the flow and steps. If you only modified a single step, use the \`test_run_step\` tool to test it. If you modified the flow, use the \`test_run_flow\` tool to test it. If the user cancels the test run, do not try again and wait for the next user instruction.
When testing steps that are sql scripts, the arguments to be passed are { database: $res:<db_resource> }.

### Inline Script References (Token Optimization)

To reduce token usage, rawscript content in the flow you receive is replaced with references in the format \`inline_script.{module_id}\`. For example:

\`\`\`json
{
  "id": "step_a",
  "value": {
    "type": "rawscript",
    "content": "inline_script.step_a",
    "language": "bun"
  }
}
\`\`\`

**To modify existing script code:**
- Use \`set_module_code\` tool for code-only changes: \`set_module_code({ moduleId: "step_a", code: "..." })\`

**To add a new inline script module:**
- Use \`add_module\` with the full code content directly (not a reference)
- Avoid coding in single lines, always use multi-line code blocks.
- The system will automatically store and optimize it

**To inspect existing code:**
- Use \`inspect_inline_script\` tool to view the current code: \`inspect_inline_script({ moduleId: "step_a" })\`

### Input Transforms for Rawscripts

Rawscript modules use \`input_transforms\` to map function parameters to values. Each key in \`input_transforms\` corresponds to a parameter name in your script's \`main\` function.

**Transform Types:**
- \`static\`: Fixed value passed directly
- \`javascript\`: Dynamic expression evaluated at runtime

**Available Variables in JavaScript Expressions:**
- \`flow_input.{property}\` - Access flow input parameters
- \`results.{step_id}\` - Access output from a previous step
- \`flow_input.iter.value\` - Current item when inside a for-loop
- \`flow_input.iter.index\` - Current index when inside a for-loop

**Example - Rawscript using flow input and previous step result:**
\`\`\`json
{
  "id": "step_b",
  "value": {
    "type": "rawscript",
    "language": "bun",
    "content": "export async function main(userId: string, data: any[]) {
		return "Hello, world!";
	}",
    "input_transforms": {
      "userId": {
        "type": "javascript",
        "expr": "flow_input.user_id"
      },
      "data": {
        "type": "javascript",
        "expr": "results.step_a"
      }
    }
  }
}
\`\`\`

**Example - Static value:**
\`\`\`json
{
  "input_transforms": {
    "limit": {
      "type": "static",
      "value": 100
    }
  }
}
\`\`\`

**Important:** The parameter names in \`input_transforms\` must match the function parameter names in your script. When you create or modify a rawscript, always define \`input_transforms\` to connect it to flow inputs or results from other steps.

### Other Key Concepts
- **Resources**: For flow inputs, use type "object" with format "resource-<type>". For step inputs, use "$res:path/to/resource"
- **Module IDs**: Must be unique and valid identifiers. Used to reference results via \`results.step_id\`
- **Module types**: Use 'bun' as default language for rawscript if unspecified

### Writing Code for Modules

**IMPORTANT: Before writing any code for a rawscript module, you MUST call the \`get_instructions_for_code_generation\` tool with the target language.** This tool provides essential language-specific instructions.

Always call this tool first when:
- Creating a new rawscript module
- Modifying existing code in a module
- Setting code via \`set_module_code\`

Example: Before writing TypeScript/Bun code, call \`get_instructions_for_code_generation({ language: "bun" })\`

### Creating New Steps

1. **Search for existing scripts first** (unless user explicitly asks to write from scratch):
   - First: \`search_scripts\` to find workspace scripts
   - Then: \`search_hub_scripts\` (only consider highly relevant results)
   - Only create a raw script if no suitable script is found

2. **Add the module using \`add_module\`:**
   - If using existing script: \`add_module({ afterId: "previous_step", value: { id: "new_step", value: { type: "script", path: "f/folder/script" } } })\`
   - If creating rawscript:
     - Default language is 'bun' if not specified
     - **First call \`get_instructions_for_code_generation\` to get the correct code format**
     - Include full code in the content field
     - Always define \`input_transforms\` to connect parameters to flow inputs or previous step results

3. **Update flow schema if needed:**
   - If your module references flow_input properties that don't exist yet, add them using \`set_flow_schema\`

### AI Agent Tools

AI agents can use tools to accomplish tasks. To manage tools for an AI agent:

- **Adding a tool to an AI agent**: Use \`add_module\` with \`insideId\` set to the agent's ID and \`branchPath: "tools"\`
  - Tool order doesn't affect execution, so you can omit \`afterId\` (defaults to inserting at beginning)
  - Example: \`add_module({ insideId: "ai_agent_step", branchPath: "tools", value: { id: "search_docs", summary: "Search documentation", value: { tool_type: "flowmodule", type: "rawscript", language: "bun", content: "...", input_transforms: {} } } })\`

- **Removing a tool from an AI agent**: Use \`remove_module\` with the tool's ID
  - The tool will be found and removed from the agent's tools array

- **Modifying a tool**: Use \`modify_module\` with the tool's ID
  - Example: \`modify_module({ id: "search_docs", value: { ... } })\`

- **Tool IDs**: Cannot contain spaces - use underscores (e.g., \`get_user_data\` not \`get user data\`)
- **Tool summaries**: Unlike other module summaries, tool summaries cannot contain spaces, use underscores instead.

- **Tool types**:
  - \`flowmodule\`: A script/flow that the agent can call (same as regular flow modules but with \`tool_type: "flowmodule"\`)
  - \`mcp\`: Reference to an MCP server tool

**Example - Adding a rawscript tool to an agent:**
\`\`\`json
add_module({
  insideId: "my_agent",
  branchPath: "tools",
  value: {
    id: "fetch_weather",
    summary: "Get current weather for a location",
    value: {
      tool_type: "flowmodule",
      type: "rawscript",
      language: "bun",
      content: "export async function main(location: string) { ... }",
      input_transforms: {
        location: { type: "static", value: "" }
      }
    }
  }
})
\`\`\`

## Resource Types
On Windmill, credentials and configuration are stored in resources. Resource types define the format of the resource.
- Use the \`resource_type\` tool to search for available resource types (e.g. stripe, google, postgresql, etc.)
- If the user needs a resource as flow input, set the property type in the schema to "object" and add a key called "format" set to "resource-nameofresourcetype" (e.g. "resource-stripe")
- If the user wants a specific resource as step input, set the step value to a static string in the format: "$res:path/to/resource"

### Contexts

You have access to the following contexts:
- Database schemas: Schema of databases the user is using
- Flow diffs: Diff between current flow and last deployed flow
- Focused flow modules: IDs of modules the user is focused on. Your response should focus on these modules
`

/**
 * Get the production tool by name
 */
function getProductionTool(name: string): Tool<FlowAIChatHelpers> | undefined {
	return flowTools.find((t) => t.def.function.name === name)
}

/**
 * Build the tools array for the no-full-schema variant.
 * Uses all production tools except add_module and modify_module,
 * which are replaced with simplified schema versions.
 * Also adds a dedicated add_branch tool for clearer branch creation.
 */
function buildNoSchemaTools(): Tool<FlowAIChatHelpers>[] {
	const productionAddModule = getProductionTool('add_module')
	const productionModifyModule = getProductionTool('modify_module')

	if (!productionAddModule || !productionModifyModule) {
		throw new Error('Could not find add_module or modify_module in production tools')
	}

	// Get all production tools except add_module and modify_module
	const otherTools = flowTools.filter(
		(t) => t.def.function.name !== 'add_module' && t.def.function.name !== 'modify_module'
	)

	// Create custom tools with simplified schemas but same implementations
	const customAddModule: Tool<FlowAIChatHelpers> = {
		...productionAddModule,
		def: noSchemaAddModuleToolDef
	}

	const customModifyModule: Tool<FlowAIChatHelpers> = {
		...productionModifyModule,
		def: noSchemaModifyModuleToolDef
	}

	// Create the add_branch tool with custom implementation
	const addBranchTool: Tool<FlowAIChatHelpers> = {
		def: addBranchToolDef,
		fn: addBranchImpl
	}

	return [...otherTools, customAddModule, customModifyModule, addBranchTool]
}

/**
 * No-Full-Schema Variant
 *
 * Uses simplified tool schemas that avoid circular references.
 * Container types (loops, branches, agents) must have empty modules arrays,
 * and the LLM must use separate add_module calls to add steps inside.
 */
export const NO_FULL_SCHEMA_VARIANT: VariantConfig = {
	name: 'no-full-schema',
	description:
		'Simplified tool schemas without circular refs. Containers require nested add_module calls.',
	systemPrompt: {
		type: 'custom',
		content: NO_FULL_SCHEMA_SYSTEM_PROMPT
	},
	tools: {
		type: 'custom',
		tools: buildNoSchemaTools()
	}
}

# OpenFlow Schema

The OpenFlow schema defines the structure of Windmill flows.

## OpenFlow

Top-level flow definition containing metadata, configuration, and the flow structure

**Properties:**

- `summary`: string (required) - Short description of what this flow does
- `description`: string - Detailed documentation for this flow
- `value`: FlowValue (required)
- `schema`: object - JSON Schema for flow inputs. Use this to define input parameters, their types, defaults, and validation. For resource inputs, set type to 'object' and format to 'resource-<type>' (e.g., 'resource-stripe')

## FlowValue

The flow structure containing modules and optional preprocessor/failure handlers

**Properties:**

- `modules`: array (required) - Array of steps that execute in sequence. Each step can be a script, subflow, loop, or branch
- `failure_module`: FlowModule - Special module that executes when the flow fails. Receives error object with message, name, stack, and step_id. Must have id 'failure'. Only supports script/rawscript types
- `preprocessor_module`: FlowModule - Special module that runs before the first step on external triggers. Must have id 'preprocessor'. Only supports script/rawscript types. Cannot reference other step results
- `same_worker`: boolean - If true, all steps run on the same worker for better performance
- `concurrent_limit`: number - Maximum number of concurrent executions of this flow
- `concurrency_key`: string - Expression to group concurrent executions (e.g., by user ID)
- `concurrency_time_window_s`: number - Time window in seconds for concurrent_limit
- `debounce_delay_s`: number - Delay in seconds to debounce flow executions
- `debounce_key`: string - Expression to group debounced executions
- `skip_expr`: string - JavaScript expression to conditionally skip the entire flow
- `cache_ttl`: number - Cache duration in seconds for flow results
- `cache_ignore_s3_path`: boolean
- `flow_env`: object - Environment variables available to all steps
- `priority`: number - Execution priority (higher numbers run first)
- `early_return`: string - JavaScript expression to return early from the flow
- `chat_input_enabled`: boolean - Whether this flow accepts chat-style input
- `notes`: array - Sticky notes attached to the flow

## FlowModule

A single step in a flow. Can be a script, subflow, loop, or branch

**Properties:**

- `id`: string (required) - Unique identifier for this step. Used to reference results via 'results.step_id'. Must be a valid identifier (alphanumeric, underscore, hyphen)
- `value`: FlowModuleValue (required)
- `stop_after_if`: StopAfterIf - Early termination condition evaluated after this step completes
- `stop_after_all_iters_if`: StopAfterIf - For loops only - early termination condition evaluated after all iterations complete
- `skip_if`: object - Conditionally skip this step based on previous results or flow inputs
- `sleep`: InputTransform - Delay before executing this step (in seconds or as expression)
- `cache_ttl`: number - Cache duration in seconds for this step's results
- `cache_ignore_s3_path`: boolean
- `timeout`: InputTransform - Maximum execution time in seconds (static value or expression)
- `delete_after_use`: boolean - If true, this step's result is deleted after use to save memory
- `summary`: string - Short description of what this step does
- `mock`: object - Mock configuration for testing without executing the actual step
- `suspend`: object - Configuration for approval/resume steps that wait for user input
- `priority`: number - Execution priority for this step (higher numbers run first)
- `continue_on_error`: boolean - If true, flow continues even if this step fails
- `retry`: Retry - Retry configuration if this step fails

## FlowModuleValue

The actual implementation of a flow step. Can be a script (inline or referenced), subflow, loop, branch, or special module type

**One of:**

- RawScript
- PathScript
- PathFlow
- ForloopFlow
- WhileloopFlow
- BranchOne
- BranchAll
- Identity
- AiAgent

## InputTransform

Maps input parameters for a step. Can be a static value or a JavaScript expression that references previous results or flow inputs

**One of:**

- StaticTransform
- JavascriptTransform

## StaticTransform

Static value passed directly to the step. Use for hardcoded values or resource references like '$res:path/to/resource'

**Properties:**

- `value`:  - The static value. For resources, use format '$res:path/to/resource'
- `type`: string (required)

## JavascriptTransform

JavaScript expression evaluated at runtime. Can reference previous step results via 'results.step_id' or flow inputs via 'flow_input.property'. Inside loops, use 'flow_input.iter.value' for the current iteration value

**Properties:**

- `expr`: string (required) - JavaScript expression returning the value. Available variables - results (object with all previous step results), flow_input (flow inputs), flow_input.iter (in loops)
- `type`: string (required)

## RawScript

Inline script with code defined directly in the flow. Use 'bun' as default language if unspecified. The script receives arguments from input_transforms

**Properties:**

- `input_transforms`: object (required) - Map of parameter names to their values (static or JavaScript expressions). These become the script's input arguments
- `content`: string (required) - The script source code. Should export a 'main' function
- `language`: string (required) - Programming language for this script
- `path`: string - Optional path for saving this script
- `lock`: string - Lock file content for dependencies
- `type`: string (required)
- `tag`: string - Worker group tag for execution routing
- `concurrent_limit`: number - Maximum concurrent executions of this script
- `concurrency_time_window_s`: number - Time window for concurrent_limit
- `custom_concurrency_key`: string - Custom key for grouping concurrent executions
- `is_trigger`: boolean - If true, this script is a trigger that can start the flow
- `assets`: array - External resources this script accesses (S3 objects, resources, etc.)

## PathScript

Reference to an existing script by path. Use this when calling a previously saved script instead of writing inline code

**Properties:**

- `input_transforms`: object (required) - Map of parameter names to their values (static or JavaScript expressions). These become the script's input arguments
- `path`: string (required) - Path to the script in the workspace (e.g., 'f/scripts/send_email')
- `hash`: string - Optional specific version hash of the script to use
- `type`: string (required)
- `tag_override`: string - Override the script's default worker group tag
- `is_trigger`: boolean - If true, this script is a trigger that can start the flow

## PathFlow

Reference to an existing flow by path. Use this to call another flow as a subflow

**Properties:**

- `input_transforms`: object (required) - Map of parameter names to their values (static or JavaScript expressions). These become the subflow's input arguments
- `path`: string (required) - Path to the flow in the workspace (e.g., 'f/flows/process_user')
- `type`: string (required)

## ForloopFlow

Executes nested modules in a loop over an iterator. Inside the loop, use 'flow_input.iter.value' to access the current iteration value, and 'flow_input.iter.index' for the index. Supports parallel execution for better performance on I/O-bound operations

**Properties:**

- `modules`: array (required) - Steps to execute for each iteration. These can reference the iteration value via 'flow_input.iter.value'
- `iterator`: InputTransform (required) - JavaScript expression that returns an array to iterate over. Can reference 'results.step_id' or 'flow_input'
- `skip_failures`: boolean (required) - If true, iteration failures don't stop the loop. Failed iterations return null
- `type`: string (required)
- `parallel`: boolean - If true, iterations run concurrently (faster for I/O-bound operations). Use with parallelism to control concurrency
- `parallelism`: InputTransform - Maximum number of concurrent iterations when parallel=true. Limits resource usage. Can be static number or expression
- `squash`: boolean

## WhileloopFlow

Executes nested modules repeatedly while a condition is true. The loop checks the condition after each iteration. Use stop_after_if on modules to control loop termination

**Properties:**

- `modules`: array (required) - Steps to execute in each iteration. Use stop_after_if to control when the loop ends
- `skip_failures`: boolean (required) - If true, iteration failures don't stop the loop. Failed iterations return null
- `type`: string (required)
- `parallel`: boolean - If true, iterations run concurrently (use with caution in while loops)
- `parallelism`: InputTransform - Maximum number of concurrent iterations when parallel=true
- `squash`: boolean

## BranchOne

Conditional branching where only the first matching branch executes. Branches are evaluated in order, and the first one with a true expression runs. If no branches match, the default branch executes

**Properties:**

- `branches`: array (required) - Array of branches to evaluate in order. The first branch with expr evaluating to true executes
- `default`: array (required) - Steps to execute if no branch expressions match
- `type`: string (required)

## BranchAll

Parallel branching where all branches execute simultaneously. Unlike BranchOne, all branches run regardless of conditions. Useful for executing independent tasks concurrently

**Properties:**

- `branches`: array (required) - Array of branches that all execute (either in parallel or sequentially)
- `type`: string (required)
- `parallel`: boolean - If true, all branches execute concurrently. If false, they execute sequentially

## Identity

Pass-through module that returns its input unchanged. Useful for flow structure or as a placeholder

**Properties:**

- `type`: string (required)
- `flow`: boolean - If true, marks this as a flow identity (special handling)

## Retry

Retry configuration for failed module executions

**Properties:**

- `constant`: object - Retry with constant delay between attempts
- `exponential`: object - Retry with exponential backoff (delay doubles each time)
- `retry_if`: RetryIf

## StopAfterIf

Early termination condition for a module

**Properties:**

- `skip_if_stopped`: boolean - If true, following steps are skipped when this condition triggers
- `expr`: string (required) - JavaScript expression evaluated after the module runs. Can use 'result' (step's result) or 'flow_input'. Return true to stop
- `error_message`: string - Custom error message shown when stopping


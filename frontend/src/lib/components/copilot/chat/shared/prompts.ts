/**
 * Shared system prompts for copilot functionality
 *
 * This module imports from the central system_prompts source of truth
 * and provides convenience exports for frontend copilot components.
 */

// Re-export all prompts from the central source
export {
	SCRIPT_BASE,
	FLOW_BASE,
	LANG_TYPESCRIPT,
	LANG_PYTHON,
	LANG_SQL,
	LANG_PHP,
	LANG_RUST,
	LANG_GO,
	LANG_BASH,
	LANG_POWERSHELL,
	LANG_GRAPHQL,
	LANG_COMPILED,
	RESOURCE_TYPES,
	S3_OBJECTS,
	SDK_TYPESCRIPT,
	SDK_PYTHON,
	OPENFLOW_SCHEMA
} from '$system_prompts/prompts'

// Re-export helper functions
export { getScriptPrompt, getFlowPrompt } from '$system_prompts/index'

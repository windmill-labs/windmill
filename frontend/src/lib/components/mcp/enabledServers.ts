import { createPathsPreference } from '$lib/components/copilot/chat/enabledPathsPreference'

/** Which MCP servers the chat may act through, per workspace and per account. A
 * server's tools both reach an external system and put their descriptions in the
 * model's context, so one is off until it is turned on; connecting one through the
 * chat turns it on for the person who connected it. */
const preference = createPathsPreference('wm_mcp_enabled', false)

/** Servers are off by default, so what is stored as on is the whole enabled set. */
export const enabledMcpPaths = preference.explicitlyEnabledPaths
export const isMcpEnabled = preference.isEnabled
export const setMcpEnabled = preference.setEnabled

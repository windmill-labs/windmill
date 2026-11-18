import type {
	SessionAccess,
	SessionCapability
} from '$lib/components/copilot/chat/sessionCapabilities'
import type { McpAccess } from './McpAccessPicker.svelte'

const BUILDS: [SessionCapability, string][] = [
	['write_draft', 'scripts'],
	['write_flow_draft', 'flows'],
	['write_app_draft', 'apps'],
	['write_schedule_draft', 'schedules'],
	['write_trigger_draft', 'triggers']
]

/** How a workspace narrows the access picked for a token, read from the session access the chat
 * resolves there (`resolveSessionAccess`), the twin of what the MCP server holds the token to. */
export function accessNarrowing(picked: McpAccess, there: SessionAccess): string | undefined {
	if (picked === 'read') return undefined
	if (!there.has('run'))
		return picked === 'selected'
			? "your role can't run anything, so the client gets no tools"
			: "your role can't run anything, so the client only reads"
	if (picked !== 'full') return undefined
	if (!there.has('modify_workspace'))
		return 'it is protected against direct changes, so the client only reads and runs'
	const cannot = BUILDS.filter(([cap]) => !there.has(cap)).map(([, what]) => what)
	const parts: string[] = []
	if (cannot.length > 0) parts.push(`your role can't build ${listed(cannot)}`)
	if (!there.has('deploy')) parts.push("the deploy rules don't let you deploy")
	if (parts.length === 0) return undefined
	return `${parts.join(' and ')}, so the client can't either`
}

function listed(items: string[]): string {
	return items.length === 1
		? items[0]
		: `${items.slice(0, -1).join(', ')} or ${items[items.length - 1]}`
}

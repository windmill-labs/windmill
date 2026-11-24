import type { DisplayMessage } from '$lib/components/copilot/chat/shared'
import type { AgentTurn } from './types'

/**
 * A recorded conversation as the AI chat draws one: each turn's message, the tools the agent
 * called, then its answer. The recorder and the Hub page both draw from this, so what is
 * recorded is what a visitor reads, tool arguments and results included.
 */
export function agentRecordingMessages(turns: AgentTurn[]): DisplayMessage[] {
	const out: DisplayMessage[] = []
	turns.forEach((turn, i) => {
		out.push({ role: 'user', index: out.length, content: turn.user_message })
		;(turn.tool_calls ?? []).forEach((call, j) =>
			out.push({
				role: 'tool',
				tool_call_id: `${i}-${j}`,
				content: call.name,
				toolName: call.name,
				parameters: call.args,
				result: call.result,
				// As the agent chat does: the row opens on what the tool got and returned.
				showDetails: call.args != null || call.result != null
			})
		)
		out.push({ role: 'assistant', content: turn.answer })
	})
	return out
}

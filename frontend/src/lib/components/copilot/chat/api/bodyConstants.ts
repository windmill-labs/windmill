import type { EndpointTool } from '$lib/gen/types.gen'

/**
 * Add the body fields an endpoint fills in for its callers rather than exposing.
 * They are deliberately absent from `body_schema` — a field whose only correct
 * value is fixed invites a model to place a placeholder there — so a body
 * assembled from that schema alone omits them and the call reaches the API
 * incomplete. Mirrors what the MCP server does for its own callers.
 */
export function withBodyConstants(endpoint: EndpointTool, body: unknown): unknown {
	if (
		!endpoint.body_constants ||
		typeof body !== 'object' ||
		body === null ||
		Array.isArray(body)
	) {
		return body
	}
	return { ...body, ...endpoint.body_constants }
}

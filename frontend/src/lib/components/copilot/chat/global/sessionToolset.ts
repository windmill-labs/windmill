import type { ChatCompletionFunctionTool } from 'openai/resources/index.mjs'
import type { SessionAccess, SessionToolPolicy } from '../sessionCapabilities'

type FilterableTool = {
	def: ChatCompletionFunctionTool
	requires?: SessionToolPolicy
	kindRequires?: Readonly<Record<string, SessionToolPolicy>>
}

const satisfies = (policy: SessionToolPolicy, access: SessionAccess) =>
	policy.every((c) => access.has(c))

/** Fails closed: every array that feeds a session is typed to declare `requires`, so a tool
 * without one arrived some other way, and leaking it is the worse of the two answers. */
export function sessionToolAllowed(tool: FilterableTool, access: SessionAccess): boolean {
	return tool.requires ? satisfies(tool.requires, access) : false
}

/** Filter an assembled toolset, and cut each kind-taking tool's `type` enum to the kinds
 * `access` can land. `access` undefined means "not resolved yet, or not a session" — the
 * toolset passes through untouched. */
export function filterSessionTools<T extends FilterableTool>(
	tools: T[],
	access: SessionAccess | undefined
): T[] {
	if (!access) return tools
	return tools.filter((t) => sessionToolAllowed(t, access)).map((t) => narrowKinds(t, access))
}

/** Returns the tool itself when every kind is allowed, so full access ships the same bytes:
 * tool defs are part of every iteration's cached prefix. Builds new objects otherwise — the
 * defs are module singletons shared by every session. */
function narrowKinds<T extends FilterableTool>(tool: T, access: SessionAccess): T {
	const kindRequires = tool.kindRequires
	if (!kindRequires) return tool
	const parameters = tool.def.function.parameters as {
		properties: Record<string, { enum?: string[] }>
	}
	const kinds = parameters.properties.type.enum ?? []
	// A kind with no entry is withheld, as a tool with no `requires` is.
	const allowed = kinds.filter((k) => kindRequires[k] && satisfies(kindRequires[k], access))
	if (allowed.length === kinds.length) return tool
	const def = structuredClone(tool.def)
	;(def.function.parameters as typeof parameters).properties.type.enum = allowed
	return { ...tool, def }
}

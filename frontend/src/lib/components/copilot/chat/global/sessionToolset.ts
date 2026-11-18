import type { ChatCompletionFunctionTool } from 'openai/resources/index.mjs'
import { isReadOnlyAccess, type SessionAccess, type SessionToolPolicy } from '../sessionCapabilities'

type FilterableTool = {
	def: ChatCompletionFunctionTool
	requires?: SessionToolPolicy
	kindRequires?: Readonly<Partial<Record<string, SessionToolPolicy>>>
	planModeSafe?: boolean
	readOnlySafe?: boolean
}

const satisfies = (policy: SessionToolPolicy, access: SessionAccess) =>
	policy.every((c) => access.has(c))

/** Fails closed: every array that feeds a session is typed to declare `requires`, so a tool
 * without one arrived some other way, and leaking it is the worse of the two answers. */
export function sessionToolAllowed(tool: FilterableTool, access: SessionAccess): boolean {
	if (!tool.requires || !satisfies(tool.requires, access)) return false
	return !isReadOnlyAccess(access) || readOnlyToolAllowed(tool)
}

/** A read-only session keeps the reads `planModeSafe` already marks, plus what opts in with
 * `readOnlySafe`. Keyed on the tags rather than a list of writes, so a tool added untagged
 * stays out of a production workspace until someone decides it belongs there. */
function readOnlyToolAllowed(tool: FilterableTool): boolean {
	return tool.planModeSafe === true || tool.readOnlySafe === true
}

/** Filter an assembled toolset, and cut each kind-taking tool's `type` enum to the kinds
 * `access` can land. `access` undefined means "not resolved yet, or not a session" — the
 * toolset passes through untouched. */
export function filterSessionTools<T extends FilterableTool>(
	tools: T[],
	access: SessionAccess | undefined
): T[] {
	if (!access) return tools
	return tools
		.filter((t) => sessionToolAllowed(t, access))
		.flatMap((t) => narrowKinds(t, access) ?? [])
}

/** Returns the tool itself when every kind is allowed, so full access ships the same bytes:
 * tool defs are part of every iteration's cached prefix. Builds new objects otherwise — the
 * defs are module singletons shared by every session. `undefined` when no kind is left: an
 * empty enum would offer a tool no call can satisfy. */
function narrowKinds<T extends FilterableTool>(tool: T, access: SessionAccess): T | undefined {
	const kindRequires = tool.kindRequires
	if (!kindRequires) return tool
	const parameters = tool.def.function.parameters as {
		properties: Record<string, { enum?: string[] }>
	}
	const kinds = parameters.properties.type.enum ?? []
	// A kind with no entry is withheld, as a tool with no `requires` is.
	const allowed = kinds.filter((k) => kindRequires[k] && satisfies(kindRequires[k], access))
	if (allowed.length === kinds.length) return tool
	if (allowed.length === 0) return undefined
	const def = structuredClone(tool.def)
	;(def.function.parameters as typeof parameters).properties.type.enum = allowed
	return { ...tool, def }
}

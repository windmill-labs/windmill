import { describe, expect, it } from 'vitest'
import {
	agentSteps,
	attachmentsTargetFor,
	flowInputRef,
	resolveAgentChatInputs
} from './agentAttachmentInput'
import type { FlowModule } from '$lib/gen'

/** An agent step with the given input transforms, as the editor stores them. */
function agentWith(input_transforms: Record<string, any>, id = 'a'): FlowModule {
	return {
		id,
		value: { type: 'aiagent', tools: [], input_transforms }
	} as unknown as FlowModule
}

describe('agentSteps', () => {
	it('finds agents inside loops and branches', () => {
		const modules = [
			{ id: 'loop', value: { type: 'forloopflow', modules: [agentWith({}, 'in-loop')] } },
			{
				id: 'branch',
				value: {
					type: 'branchone',
					default: [agentWith({}, 'in-default')],
					branches: [{ modules: [agentWith({}, 'in-branch')] }]
				}
			},
			{
				id: 'all',
				value: { type: 'branchall', branches: [{ modules: [agentWith({}, 'in-all')] }] }
			}
		] as unknown as FlowModule[]
		expect(agentSteps(modules).map((m) => m.id)).toEqual([
			'in-loop',
			'in-default',
			'in-branch',
			'in-all'
		])
	})

	// The graph walks an agent's tools as child steps; a tool agent's inputs belong to the
	// agent that calls it, not to the chat.
	it("ignores an agent carried as another agent's tool", () => {
		const parent = agentWith({})
		;(parent.value as any).tools = [
			{ id: 'summarize', value: { tool_type: 'flowmodule', type: 'aiagent', tools: [] } }
		]
		expect(agentSteps([parent]).map((m) => m.id)).toEqual(['a'])
	})
})

describe('flowInputRef', () => {
	it('names the one input an expression reads, however it reshapes it', () => {
		expect(
			flowInputRef({
				type: 'javascript',
				expr: '(flow_input.files || []).map(f => ({ bucket: f.storage, key: f.s3 }))'
			})
		).toBe('files')
		expect(flowInputRef({ type: 'javascript', expr: 'flow_input?.docs' })).toBe('docs')
	})

	it('names nothing for two inputs or a static value', () => {
		expect(
			flowInputRef({ type: 'javascript', expr: '[...flow_input.a, ...flow_input.b]' })
		).toBeUndefined()
		expect(flowInputRef({ type: 'static', value: [] })).toBeUndefined()
	})
})

describe('resolveAgentChatInputs', () => {
	const schema = { properties: { files: { type: 'array' } }, required: [] }
	const reader = (name: string) =>
		agentWith({ user_attachments: { type: 'javascript', expr: `flow_input.${name}` } })
	// Every agent step carries a placeholder transform for each key of AI_AGENT_SCHEMA
	// (loadSchemaFromModule writes them back onto the module), so an agent that reads
	// nothing must not be mistaken for one reading a different input.
	const seeded = () => agentWith({ user_attachments: { type: 'static', value: undefined } })

	it('promotes the input one agent reads', () => {
		expect(resolveAgentChatInputs([reader('files')], schema).map((i) => i.name)).toEqual(['files'])
	})

	it('still promotes it when another agent leaves the field unwired', () => {
		expect(resolveAgentChatInputs([reader('files'), seeded()], schema).map((i) => i.name)).toEqual([
			'files'
		])
	})

	it('promotes nothing when two agents read different inputs', () => {
		const twoInputs = {
			properties: { files: { type: 'array' }, docs: { type: 'array' } },
			required: []
		}
		expect(resolveAgentChatInputs([reader('files'), reader('docs')], twoInputs)).toEqual([])
	})

	it('promotes nothing for an input the schema does not declare', () => {
		expect(resolveAgentChatInputs([reader('missing')], schema)).toEqual([])
	})
})

describe('attachmentsTargetFor', () => {
	const input = (property: Record<string, any>) =>
		({ name: 'files', key: 'user_attachments', property }) as any

	it('takes a list of s3 files, and says it holds several', () => {
		expect(
			attachmentsTargetFor(input({ type: 'array', items: { resourceType: 's3object' } }))
		).toEqual({ name: 'files', multiple: true })
	})

	it('takes a single s3 file', () => {
		expect(attachmentsTargetFor(input({ format: 'resource-s3_object' }))).toEqual({
			name: 'files',
			multiple: false
		})
	})

	// The transform can build the s3 object itself, promoting an input that holds a key
	// rather than a file. Uploading into it would write an object where a string is declared.
	it('offers no paperclip where the input cannot hold a file', () => {
		expect(attachmentsTargetFor(input({ type: 'string' }))).toBeUndefined()
		expect(
			attachmentsTargetFor(input({ type: 'array', items: { type: 'string' } }))
		).toBeUndefined()
		expect(attachmentsTargetFor(undefined)).toBeUndefined()
	})
})

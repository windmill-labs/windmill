import { z } from 'zod'
import { createToolDef, type Tool } from '../shared'
import { summarizeTasks, type SessionTasksStore } from './tasksState.svelte'

// The subset of GlobalToolHelpers these tools read. Kept local (not imported from
// global/core) so the tools don't pull the whole global tool module — which would be a
// circular import, since global/core registers these tools.
type TaskToolHelpers = {
	tasks?: SessionTasksStore
	sessionId?: string
}

const MAX_TASKS_PER_CALL = 20

const createTasksSchema = z.object({
	tasks: z
		.array(
			z.object({
				subject: z.string().describe('Brief imperative title, e.g. "Fix the auth redirect".'),
				description: z.string().describe('What needs to be done.'),
				activeForm: z
					.string()
					.optional()
					.describe(
						'Present continuous form shown while this task runs, e.g. "Fixing the auth redirect".'
					)
			})
		)
		.describe('The tasks to add, in the order they should be worked on.')
})

const updateTaskSchema = z.object({
	id: z.number().int().describe('Id of the task to update, as returned by create_tasks.'),
	status: z
		.enum(['pending', 'in_progress', 'completed', 'deleted'])
		.optional()
		.describe('New status. "deleted" drops the task from the plan.'),
	subject: z.string().optional().describe('New title.'),
	description: z.string().optional().describe('New description.'),
	activeForm: z.string().optional().describe('New present continuous form.')
})

const listTasksSchema = z.object({})

const UNAVAILABLE = 'Tasks are only available inside an AI session.'

type ToolRun = Parameters<Tool<{}>['fn']>[0]
type TaskCtx = {
	tasks: SessionTasksStore
	sessionId: string
	/** Report an error on the transcript card and to the model in one step. */
	fail: (error: string) => string
	/** The plan's one-line state, read back after a write. */
	summary: () => Promise<string>
	/** Note what the call did on the transcript card. */
	note: (content: string) => void
}

/**
 * Run `body` with the session's task store resolved, or fail closed: these tools are
 * offered to session chats only, so anywhere else there is no plan to act on.
 */
function withTasks(body: (ctx: TaskCtx, run: ToolRun) => Promise<string>) {
	return async (run: ToolRun): Promise<string> => {
		const { toolId, toolCallbacks } = run
		const fail = (error: string) => {
			toolCallbacks.setToolStatus(toolId, { content: error, error })
			return JSON.stringify({ success: false, error })
		}
		const { tasks, sessionId } = (run.helpers ?? {}) as TaskToolHelpers
		if (!tasks || !sessionId) return fail(UNAVAILABLE)
		return body(
			{
				tasks,
				sessionId,
				fail,
				summary: async () => summarizeTasks(await tasks.listForSession(sessionId)),
				note: (content) => toolCallbacks.setToolStatus(toolId, { content })
			},
			run
		)
	}
}

export const taskTools: Tool<{}>[] = [
	{
		def: createToolDef(
			createTasksSchema,
			'create_tasks',
			"Add tasks to the current session's plan, all at once, in the order they should be worked on. Use for work spanning three or more distinct steps; skip it for a single straightforward change. All tasks start as pending — call update_task to set one in_progress before starting it."
		),
		showDetails: true,
		fn: withTasks(async (ctx, { args }) => {
			const { tasks } = createTasksSchema.parse(args)
			if (tasks.length === 0) return ctx.fail('Provide at least one task.')
			if (tasks.length > MAX_TASKS_PER_CALL) {
				return ctx.fail(
					`Too many tasks (${tasks.length}, limit ${MAX_TASKS_PER_CALL}). Plan the next few steps instead.`
				)
			}
			const created = await ctx.tasks.createMany(ctx.sessionId, tasks)
			ctx.note(`Added ${created.length} task${created.length === 1 ? '' : 's'}`)
			return JSON.stringify({
				success: true,
				ids: created.map((t) => t.seq),
				summary: await ctx.summary()
			})
		})
	},
	{
		def: createToolDef(
			updateTaskSchema,
			'update_task',
			"Update one task in the current session's plan: mark it in_progress before starting, completed once it is genuinely done, or revise its wording. Keep it in_progress if you hit an error or could not finish."
		),
		showDetails: true,
		fn: withTasks(async (ctx, { args }) => {
			const { id, status, ...fields } = updateTaskSchema.parse(args)
			const notFound = `No task with id ${id} in this session.`

			if (status === 'deleted') {
				if (!(await ctx.tasks.remove(ctx.sessionId, id))) return ctx.fail(notFound)
				ctx.note(`Deleted task ${id}`)
				return JSON.stringify({ success: true, summary: await ctx.summary() })
			}
			const updated = await ctx.tasks.update(ctx.sessionId, id, { status, ...fields })
			if (!updated) return ctx.fail(notFound)
			ctx.note(`${statusVerb(updated.status)} "${updated.subject}"`)
			return JSON.stringify({ success: true, summary: await ctx.summary() })
		})
	},
	{
		def: createToolDef(
			listTasksSchema,
			'list_tasks',
			"Re-read the current session's plan (id, subject, description, status). Use it to recover the plan after a long run, when you are no longer sure what is left."
		),
		fn: withTasks(async (ctx) => {
			const items = await ctx.tasks.listForSession(ctx.sessionId)
			ctx.note(summarizeTasks(items))
			return JSON.stringify(
				items.map((t) => ({
					id: t.seq,
					subject: t.subject,
					description: t.description,
					status: t.status
				}))
			)
		})
	}
]

function statusVerb(status: string): string {
	return status === 'completed' ? 'Completed' : status === 'in_progress' ? 'Started' : 'Updated'
}

import type { AIProvider } from '$lib/gen'
import { z } from 'zod'

const chatFimResponseSchema = z.object({
	choices: z.array(
		z.object({
			message: z.object({
				content: z.string().optional()
			}),
			finish_reason: z.string().optional()
		})
	)
})

const deepseekFimResponseSchema = z.object({
	choices: z.array(
		z.object({
			text: z.string().optional(),
			finish_reason: z.string().optional()
		})
	)
})

export function parseFimCompletionChoice(
	body: unknown,
	provider: AIProvider
): { content: string | undefined; finish_reason: string | undefined } | undefined {
	if (provider === 'deepseek') {
		const parsedBody = deepseekFimResponseSchema.parse(body)
		const choice = parsedBody.choices[0]
		return choice ? { content: choice.text, finish_reason: choice.finish_reason } : undefined
	}

	const parsedBody = chatFimResponseSchema.parse(body)
	const choice = parsedBody.choices[0]
	return choice
		? { content: choice.message.content, finish_reason: choice.finish_reason }
		: undefined
}

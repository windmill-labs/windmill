// gpt-5+ and o-series reasoning models reject the legacy `max_tokens` field on
// the OpenAI/Azure Chat Completions API and require `max_completion_tokens`
// instead. The check strips any provider prefix (e.g. OpenRouter's "openai/o3")
// so it matches the bare model id, and the o-series match requires a digit after
// the "o" (o1/o3/o4-mini) so it does not catch unrelated ids like Mistral's
// "open-mistral-*" or "optimus-*".
export function requiresMaxCompletionTokens(model: string) {
	const normalizedModel = model.toLowerCase()
	const baseModel = normalizedModel.split('/').pop() ?? normalizedModel
	return baseModel.startsWith('gpt-5') || /^o\d/.test(baseModel)
}

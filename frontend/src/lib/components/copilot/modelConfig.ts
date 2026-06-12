import type { AIProviderModel } from '$lib/gen'

export function modelDisallowsSamplingParams(model: string) {
	const normalizedModel = model.toLowerCase()
	// Strip any provider prefix (e.g. OpenRouter's "openai/o3") so the
	// reasoning-model check matches the bare model id rather than the prefix.
	const baseModel = normalizedModel.split('/').pop() ?? normalizedModel
	// gpt-5+ and o-series reasoning models reject sampling params such as
	// temperature (only the default value is supported), regardless of which
	// provider/gateway routes the request — so this must stay provider-agnostic.
	// The o-series match requires a digit after the "o" (o1/o3/o4-mini) so it
	// does not catch unrelated ids like Mistral's "open-mistral-*" or "optimus-*".
	return (
		normalizedModel.includes('claude-fable-5') ||
		normalizedModel.includes('claude-opus-4-7') ||
		normalizedModel.includes('claude-opus-4-8') ||
		baseModel.startsWith('gpt-5') ||
		/^o\d/.test(baseModel)
	)
}

export function getDefaultChatTemperature(modelProvider: AIProviderModel): number | undefined {
	if (modelDisallowsSamplingParams(modelProvider.model)) {
		return undefined
	}

	return 0
}

export function getKnownModelContextWindow(model: string): number | undefined {
	if (model.includes('gpt-4.1') || model.includes('gemini')) {
		return 1000000
	} else if (model.includes('gpt-5')) {
		// GPT-5.4+ ship a ~1M context window; gpt-5 / -mini / -nano and the
		// 5.1/5.2 revisions remain at 400K (272K input + 128K output).
		const version = model.match(/gpt-5\.(\d+)/)
		if (version && Number(version[1]) >= 4) {
			return 1000000
		}
		return 400000
	} else if (model.includes('o4-mini') || model.includes('o3')) {
		return 200000
	} else if (model.includes('claude')) {
		// Sonnet 4.6+ and Opus 4.6+ ship a 1M context window at standard pricing (GA).
		// Haiku and older Claude models (3.x, 4.0, 4.1, 4.5) remain at 200K. The
		// version sits between the family name and any date/revision suffix, also
		// in Bedrock-style ids (e.g. anthropic.claude-sonnet-4-6-...-v1:0). The
		// minor is capped at two digits so date-suffixed base ids without a minor
		// (claude-sonnet-4-20250514) don't capture the date as the version.
		const version = model.match(/claude-(?:opus|sonnet)-(\d+)-(\d{1,2})(?!\d)/)
		if (
			version &&
			(Number(version[1]) > 4 || (Number(version[1]) === 4 && Number(version[2]) >= 6))
		) {
			return 1000000
		}
		return 200000
	} else if (model.includes('gpt-4o') || model.includes('llama') || model.includes('deepseek')) {
		return 128000
	} else if (model.includes('codestral')) {
		return 32000
	} else {
		return undefined
	}
}

export function getModelContextWindow(model: string) {
	// Trim/compaction logic needs a number; assume a conservative window when unknown.
	return getKnownModelContextWindow(model) ?? 128000
}

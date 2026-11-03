// The provider catalog, apart from `./lib` and its SDKs so that what only names providers or
// picks a default model loads neither.
import type { AIProvider } from '$lib/gen'

export interface AIProviderDetails {
	label: string
	defaultModels: string[]
}

// The first entry is what a new workspace is created with (see
// CreateWorkspaceInner), so each list leads with the balanced tier rather than
// the frontier model. The gpt-5 family is deprecated (retires 2026-12-11) but
// still served, so it stays in the list below the 5.6 models.
const OPENAI_MODELS = [
	'gpt-6.1-sol',
	'gpt-6-sol',
	'gpt-6-astra',
	'gpt-6-luna',
	'gpt-5.6-terra',
	'gpt-5.6-sol',
	'gpt-5.6-luna',
	'gpt-5',
	'gpt-5-mini',
	'gpt-4o',
	'gpt-4o-mini',
	'o4-mini',
	'o3'
]

/** The providers that chat. Every surface picking a model to talk to (the copilot, flow chat, an
 *  agent's text or image output, eval judges) offers exactly these. */
export type ChatAIProvider = Exclude<AIProvider, 'typesafe' | 'cloudflare'>

export const AI_PROVIDERS: Record<ChatAIProvider, AIProviderDetails> = {
	openai: {
		label: 'OpenAI',
		defaultModels: OPENAI_MODELS
	},
	anthropic: {
		label: 'Anthropic',
		defaultModels: [
			'claude-sonnet-5-5',
			'claude-opus-5-5',
			'claude-sonnet-5',
			'claude-opus-5',
			'claude-opus-4-8',
			'claude-haiku-4-5'
		]
	},
	googleai: {
		label: 'Google AI',
		defaultModels: [
			'gemini-3.6-flash',
			'gemini-3.5-flash',
			'gemini-3.1-pro-preview',
			'gemini-3.5-flash-lite',
			'gemini-3.1-flash-lite',
			'gemini-2.5-pro',
			'gemini-2.5-flash'
		]
	},
	azure_openai: {
		label: 'Azure OpenAI',
		defaultModels: OPENAI_MODELS
	},
	azure_foundry: {
		label: 'Azure AI Foundry',
		defaultModels: [
			'gpt-5.6-terra',
			'gpt-5.6-sol',
			'claude-sonnet-5',
			'claude-opus-5',
			'DeepSeek-V4-Pro',
			'Llama-3.3-70B-Instruct',
			'Phi-4'
		]
	},
	mistral: {
		label: 'Mistral',
		defaultModels: ['mistral-medium-latest', 'codestral-latest']
	},
	deepseek: {
		label: 'DeepSeek',
		defaultModels: ['deepseek-v4-pro', 'deepseek-flash']
	},
	groq: {
		label: 'Groq',
		defaultModels: ['openai/gpt-oss-120b', 'openai/gpt-oss-20b']
	},
	openrouter: {
		label: 'OpenRouter',
		defaultModels: ['meta-llama/llama-3.2-3b-instruct:free']
	},
	togetherai: {
		label: 'Together AI',
		defaultModels: ['meta-llama/Llama-3.3-70B-Instruct-Turbo']
	},
	aws_bedrock: {
		label: 'AWS Bedrock',
		defaultModels: [
			'global.anthropic.claude-sonnet-5',
			'global.anthropic.claude-opus-5',
			'global.anthropic.claude-haiku-4-5-20251001-v1:0'
		]
	},
	customai: {
		label: 'Custom AI',
		defaultModels: []
	}
}

/** Decision models answer typed questions instead of messages, so only an AI decision offers
 *  them: TypeSafe's Jev, Cloudflare's Jev-compatible Clef, and the model of OpenAI's Decisions
 *  API, which an OpenAI resource serves next to its chat models. The pinned Jev version is there
 *  for flows tuned against its probabilities. */
export const DECISION_AI_PROVIDERS: Record<
	Exclude<AIProvider, ChatAIProvider> | 'openai',
	AIProviderDetails
> = {
	openai: {
		label: 'OpenAI',
		defaultModels: ['gpt-6-luna']
	},
	typesafe: {
		label: 'TypeSafe',
		defaultModels: ['jev-latest', 'jev-1.13.0']
	},
	cloudflare: {
		label: 'Cloudflare',
		defaultModels: ['clef', 'clef-flash']
	}
}

/** Label and default models of any provider kind, chat or decision. A kind in neither list (a flow
 *  written by hand or by a newer version) is named as it is and offers no models. */
export function aiProviderDetails(provider: AIProvider): AIProviderDetails {
	return (
		(AI_PROVIDERS as Record<string, AIProviderDetails>)[provider] ??
		(DECISION_AI_PROVIDERS as Record<string, AIProviderDetails>)[provider] ?? {
			label: provider,
			defaultModels: []
		}
	)
}

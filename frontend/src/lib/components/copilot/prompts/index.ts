import { EDIT_PROMPT } from './editPrompt'
import { FIX_PROMPT } from './fixPrompt'
import { GEN_PROMPT } from './genPrompt'

export interface PromptsConfig {
	system: string
	prompts: {
		[lang: string]: {
			prompt: string
		}
	}
}

export const EDIT_CONFIG = EDIT_PROMPT as PromptsConfig

export const FIX_CONFIG = FIX_PROMPT as PromptsConfig

export const GEN_CONFIG = GEN_PROMPT as PromptsConfig

import editObj from './edit.yaml'
import fixObj from './fix.yaml'
import genObj from './gen.yaml'

export interface PromptsConfig {
	system: string
	prompts: {
		[lang: string]: {
			prompt: string
			example_description: string
			example_answer: string
			example_code: string
			example_error: string
		}
	}
}

export const EDIT_CONFIG = editObj as PromptsConfig

export const FIX_CONFIG = fixObj as PromptsConfig

export const GEN_CONFIG = genObj as PromptsConfig

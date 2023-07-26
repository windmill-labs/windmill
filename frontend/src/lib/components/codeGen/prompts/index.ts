import editObj from './edit.yaml'
import fixObj from './fix.yaml'
import genObj from './gen.yaml'

interface PromptsConfig {
	system: string
	prompts: {
		[key: string]: {
			prompt: string
		}
	}
}

const EDIT_CONFIG = editObj as PromptsConfig

const FIX_CONFIG = fixObj as PromptsConfig

const GEN_CONFIG = genObj as PromptsConfig

export { EDIT_CONFIG, FIX_CONFIG, GEN_CONFIG }

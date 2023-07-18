import editObj from './edit.yaml'
import fixObj from './fix.yaml'
import genObj from './gen.yaml'

interface CommonConfig {
	system: string
	prompt: string
}

interface GenConfig {
	system: string
	prompts: {
		[key: string]: {
			prompt: string
		}
	}
}

const EDIT_CONFIG = editObj as CommonConfig

const FIX_CONFIG = fixObj as CommonConfig

const GEN_CONFIG = genObj as GenConfig

export { EDIT_CONFIG, FIX_CONFIG, GEN_CONFIG }

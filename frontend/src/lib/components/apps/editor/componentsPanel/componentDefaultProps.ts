import type { Aligned } from "../../types"

const defaultProps = {
	inputs: {},
	componentInputs: {},
	alignable: false,
	width: 0
}

const defaultAlignement: Aligned = {
	horizontalAlignment: 'center',
	verticalAlignment: 'center'
}

export { defaultProps, defaultAlignement }

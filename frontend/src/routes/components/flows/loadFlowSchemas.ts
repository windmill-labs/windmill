import type { Schema } from '../../../common'
import { ScriptService, type Flow, type FlowModule } from '../../../gen'
import { emptySchema } from '../../../utils'

export async function loadFlowSchemas(flow: Flow, workspace: string): Promise<Schema[]> {
	const schemas = await Promise.all(
		flow.value.modules.map(async (flowModule: FlowModule) => {
			if (flowModule.value.path) {
				const script = await ScriptService.getScriptByPath({
					workspace: workspace,
					path: flowModule.value.path
				})
				return script.schema ?? emptySchema()
			} else {
				return emptySchema()
			}
		})
	)

	if (schemas.length === 0) {
		return [emptySchema()]
	}

	return schemas
}

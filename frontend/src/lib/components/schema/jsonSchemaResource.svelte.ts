import { z } from 'zod'

import { AppService } from '$lib/gen'
import { sendUserToast } from '$lib/utils'

let loadedJsonSchemaResources: Record<string, Record<string, any>> = $state({})

const jsonSchemaResourceSchema = z.object({
	schema: z.record(z.string(), z.any())
})
export async function getJsonSchemaFromResource(path: string, workspace: string) {
	if (loadedJsonSchemaResources[workspace]?.[path]) {
		return loadedJsonSchemaResources[workspace][path]
	}

	try {
		const resourceValue = await AppService.getPublicResource({
			path,
			workspace
		})
		if (!resourceValue || !resourceValue == null) {
			console.warn(`JSON schema resource ${path} not found`)
			return null
		}

		const parsedResource = jsonSchemaResourceSchema.safeParse(resourceValue)
		if (parsedResource.success) {
			const workspaceResources = loadedJsonSchemaResources[workspace]
			if (!workspaceResources) {
				loadedJsonSchemaResources[workspace] = {}
			}
			loadedJsonSchemaResources[workspace][path] = parsedResource.data.schema
			return parsedResource.data.schema
		} else {
			console.error('Invalid JSON schema resource:', parsedResource.error)
			sendUserToast(
				`Error loading json schema resource ${path}, Invalid JSON schema resource: ` +
					parsedResource.error,
				true
			)
		}
	} catch (err) {
		console.error(err)
		sendUserToast('Could not load JSON schema resource: ' + err, true)
	}
}

export async function clearJsonSchemaResourceCache(path: string, workspace: string) {
	if (loadedJsonSchemaResources[workspace]?.[path]) {
		delete loadedJsonSchemaResources[workspace][path]
	}
}

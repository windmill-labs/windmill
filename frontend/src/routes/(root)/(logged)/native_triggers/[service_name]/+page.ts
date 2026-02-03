import { getServiceConfig } from '$lib/components/triggers/native/utils'
import { error } from '@sveltejs/kit'
import type { PageLoad } from './$types'
import type { NativeServiceName } from '$lib/gen'

export const load: PageLoad = async ({ params }) => {
	const serviceName = params.service_name as NativeServiceName

	const serviceConfig = getServiceConfig(serviceName)

	if (!serviceConfig) {
		throw error(404, {
			message: `Service "${serviceName}" is not supported for native triggers.`
		})
	}

	return {
		serviceName,
		serviceConfig,
		stuff: { title: `${serviceConfig.serviceDisplayName} triggers` }
	}
}

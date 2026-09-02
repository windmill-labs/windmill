import { get } from 'svelte/store'
import { SettingsService } from './gen'
import { enterpriseLicense } from './stores'

export async function setLicense() {
	try {
		if (get(enterpriseLicense)) {
			return
		}

		const license = await SettingsService.getLicenseId()
		if (license) {
			enterpriseLicense.set(license)
		}
	} catch (e) {
		console.error('error getting license', e)
	}
}

/** Guests are an Enterprise-plan feature: the server refuses them on a Pro key, so the
 * controls that would offer them must read the plan, not merely the presence of a key. */
export function isEnterprisePlan(licenseId: string | undefined): boolean {
	return !!licenseId && !licenseId.endsWith('_pro')
}

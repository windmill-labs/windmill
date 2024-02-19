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

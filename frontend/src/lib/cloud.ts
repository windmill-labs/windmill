import { BROWSER } from 'esm-env'

export function isCloudHosted(): boolean {
	return BROWSER && window.location.hostname == 'app.windmill.dev'
}
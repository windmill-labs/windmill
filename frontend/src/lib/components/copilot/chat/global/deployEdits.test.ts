import { describe, expect, it } from 'vitest'
import { describeDeployEdits } from './deployEdits'

const at = { requestedMessage: 'v2', path: 'u/admin/a', deployedPath: 'u/admin/a' }

describe('describeDeployEdits', () => {
	it('says nothing when the card changed nothing the model asked for', () => {
		expect(describeDeployEdits(undefined, at)).toBeUndefined()
		expect(describeDeployEdits({ deploymentMessage: 'v2' }, at)).toBeUndefined()
	})

	it('names every change, a cleared message and a run-as user turned off included', () => {
		expect(
			describeDeployEdits(
				{
					deploymentMessage: '',
					deployPath: 'f/reports/a',
					onBehalfOf: { email: undefined, permissionedAs: undefined }
				},
				{ ...at, deployedPath: 'f/reports/a' }
			)
		).toBe(
			'Before confirming, the user changed the deployment message to none, the path to f/reports/a, it to run as whoever starts each run.'
		)
	})

	it('leaves out a picked path the deploy did not land at', () => {
		expect(describeDeployEdits({ deployPath: 'f/reports/a' }, at)).toBeUndefined()
	})
})

import { describe, it, expect } from 'vitest'
import type { FlowModule } from '$lib/gen'
import { describeStepSettings, stepSettingsByKey, stepSettingDefaults } from './flowStepSettings'

function step(overrides: Partial<FlowModule> = {}): FlowModule {
	return {
		id: 'a',
		value: { type: 'rawscript', language: 'bun', content: '', input_transforms: {} },
		...overrides
	} as FlowModule
}

describe('describeStepSettings', () => {
	it('reports an untouched step as configured nowhere', () => {
		expect(describeStepSettings(step()).filter((s) => s.configured)).toEqual([])
	})

	it('treats a sleep of 0 as configured and says so, rather than claiming it is off', () => {
		// Switching Sleep on seeds `{ value: 0 }`; the row must not contradict the toggle.
		const s = stepSettingsByKey(step({ sleep: { type: 'static', value: 0 } }))['sleep']
		expect(s?.configured).toBe(true)
		expect(s?.summary.text).toBe('0s after')
		expect(s?.summary.state).toBe('configured')
	})

	it('does not let an empty stop_after_if mask a configured stop_after_all_iters_if', () => {
		// Both can be set on a sequential loop.
		const s = stepSettingsByKey(
			step({
				stop_after_if: { expr: '', skip_if_stopped: false },
				stop_after_all_iters_if: { expr: 'result.done', skip_if_stopped: false }
			})
		)['early-stop']
		expect(s?.configured).toBe(true)
		expect(s?.summary.text).toBe('result.done')
	})

	it('describes retries with zero attempts instead of reporting None', () => {
		const s = stepSettingsByKey(step({ retry: { constant: { attempts: 0, seconds: 5 } } }))[
			'retries'
		]
		expect(s?.configured).toBe(true)
		expect(s?.summary.text).toBe('0 attempts, constant')
	})

	it('reads concurrency and cache from the referenced script for workspace-script steps', () => {
		const mod = step({ value: { type: 'script', path: 'u/me/s', input_transforms: {} } as any })
		const off = stepSettingsByKey(mod)
		expect(off['concurrency']?.configured).toBe(false)

		const on = stepSettingsByKey(mod, { concurrent_limit: 3, cache_ttl: 60 })
		expect(on['concurrency']?.configured).toBe(true)
		expect(on['concurrency']?.summary.text).toBe('Max 3')
		expect(on['cache']?.configured).toBe(true)
	})

	it('treats non-positive inline concurrency and cache as unset, like the runtime does', () => {
		const s = stepSettingsByKey(
			step({
				cache_ttl: -1,
				value: {
					type: 'rawscript',
					language: 'bun',
					content: '',
					input_transforms: {},
					concurrent_limit: -1
				}
			} as Partial<FlowModule>)
		)
		expect(s['concurrency']?.configured).toBe(false)
		expect(s['concurrency']?.summary.text).toBe('None')
		expect(s['cache']?.configured).toBe(false)
		expect(s['cache']?.summary.text).toBe('Off')
	})

	it('omits settings that do not apply to the step type', () => {
		const subflow = step({ value: { type: 'flow', path: 'u/me/f' } as any })
		expect(describeStepSettings(subflow).some((s) => s.key === 'concurrency')).toBe(false)
		expect(describeStepSettings(step()).some((s) => s.key === 'concurrency')).toBe(true)
	})

	it('marks an invalid retry config as invalid rather than configured', () => {
		const s = stepSettingsByKey(
			step({ retry: { exponential: { attempts: 2, multiplier: 1, seconds: -1 } } })
		)['retries']
		expect(s?.summary.state).toBe('invalid')
	})

	it('labels early stop for trigger steps by what it means there', () => {
		const trigger = step({
			value: {
				type: 'rawscript',
				language: 'bun',
				content: '',
				input_transforms: {},
				is_trigger: true
			} as any
		})
		expect(stepSettingsByKey(trigger)['early-stop']?.tooltip).toBe(
			'Stop early if there are no new events'
		)
		expect(stepSettingsByKey(step())['early-stop']?.tooltip).toBe('Early stop / break')
	})
})

describe('stepSettingDefaults', () => {
	it('seeds one predicate for trigger steps, whichever path creates them', () => {
		expect(stepSettingDefaults('early-stop', 'trigger')).toEqual({
			expr: '!result || (Array.isArray(result) && result.length == 0)',
			skip_if_stopped: true
		})
	})

	it('seeds a terminating step to stop unconditionally without marking it skipped', () => {
		expect(stepSettingDefaults('early-stop', 'end')).toEqual({
			expr: 'true',
			skip_if_stopped: false
		})
	})
})

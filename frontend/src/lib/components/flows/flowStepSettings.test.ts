import { describe, it, expect } from 'vitest'
import type { FlowModule } from '$lib/gen'
import { describeStepSettings, hasInlineConcurrency } from './flowStepSettings'

const stepSettingsByKey = (...args: Parameters<typeof describeStepSettings>) =>
	Object.fromEntries(describeStepSettings(...args).map((v) => [v.key, v]))

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

	it("prefers the module's own cache_ttl over the referenced script's, like the worker", () => {
		const mod = step({
			cache_ttl: 3600,
			value: { type: 'script', path: 'u/me/s', input_transforms: {} } as any
		})
		// No referenced settings loaded yet (the graph badges never load them), so a
		// module-level TTL has to stand on its own here.
		expect(stepSettingsByKey(mod)['cache']?.configured).toBe(true)
		expect(stepSettingsByKey(mod, { cache_ttl: 60 })['cache']?.summary.text).toBe('1 h')
	})

	it('reports a non-positive inline concurrency as invalid, not as unset', () => {
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
		// `configured` means the step carries the config, so a value the user set counts
		// even when the runtime ignores it — the summary is what says it is a no-op.
		expect(s['concurrency']?.configured).toBe(true)
		expect(s['concurrency']?.summary).toMatchObject({ text: 'Invalid limit', state: 'invalid' })
		expect(s['cache']?.configured).toBe(true)
		expect(s['cache']?.summary).toMatchObject({ text: 'No TTL set', state: 'invalid' })
	})

	it('treats a cleared concurrency input as present, not as unset', () => {
		// Emptying a number input binds `null`, not `undefined`. Reading that as unset is
		// what disabled the field the user was editing, so presence must be strict.
		const mod = step({
			value: {
				type: 'rawscript',
				language: 'bun',
				content: '',
				input_transforms: {},
				concurrent_limit: null
			}
		} as unknown as Partial<FlowModule>)
		// Presence keeps the setting editor's controls live while the field is empty.
		expect(hasInlineConcurrency(mod)).toBe(true)
		expect(stepSettingsByKey(mod)['concurrency']?.summary).toMatchObject({
			text: 'Invalid limit',
			state: 'invalid'
		})
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

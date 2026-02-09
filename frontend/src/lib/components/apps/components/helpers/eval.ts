import type { World } from '../../rx'
import { sendUserToast } from '$lib/toast'
import { waitJob } from '$lib/components/waitJob'
import { base } from '$lib/base'

export function computeGlobalContext(
	world: World | undefined,
	id: string | undefined,
	extraContext: any = {}
) {
	return {
		...Object.fromEntries(
			Object.entries(world?.outputsById ?? {})
				.filter(([k, _]) => k != 'state')
				.map(([key, value]) => {
					return [
						key,
						Object.fromEntries(Object.entries(value ?? {}).map((x) => [x[0], x[1].peak()]))
					]
				})
		),
		...extraContext,
		id
	}
}

function create_context_function_template(
	eval_string: string,
	contextKeys: string[],
	noReturn: boolean
) {
	let hasReturnAsLastLine = noReturn || eval_string.split('\n').some((x) => x.startsWith('return '))
	return `
return async function (context, state, createProxy, goto, setTab, recompute, globalRecompute, getAgGrid, setValue, setSelectedIndex, openModal, closeModal, open, close, validate, invalidate, validateAll, clearFiles, sendMessage, showToast, waitJob, askNewResource, downloadFile) {
"use strict";
${
	contextKeys && contextKeys.length > 0
		? `let ${contextKeys.map((key) => ` ${key} = createProxy('${key}', context['${key}'])`)};`
		: ``
}
${
	hasReturnAsLastLine
		? eval_string
		: `
return ${eval_string.startsWith('return ') ? eval_string.substring(7) : eval_string}`
}

}
`
}

type WmFunctor = (
	context,
	state,
	createProxy,
	goto,
	setTab,
	recompute,
	globalRecompute,
	getAgGrid,
	setValue,
	setSelectedIndex,
	openModal,
	closeModal,
	open,
	close,
	validate,
	invalidate,
	validateAll,
	clearFiles,
	sendMessage,
	showToast,
	waitJob,
	askNewResource,
	downloadFile
) => Promise<any>

let functorCache: Record<number, WmFunctor> = {}
function make_context_evaluator(eval_string, contextKeys: string[], noReturn: boolean): WmFunctor {
	let cacheKey = hashCode(JSON.stringify({ eval_string, contextKeys, noReturn }))
	if (functorCache[cacheKey]) {
		return functorCache[cacheKey]
	}
	let template = create_context_function_template(eval_string, contextKeys, noReturn)
	let functor = Function(template)
	let r = functor()
	functorCache[cacheKey] = r
	return r
}

function hashCode(s: string): number {
	var hash = 0,
		i,
		chr
	if (s.length === 0) return hash
	for (i = 0; i < s.length; i++) {
		chr = s.charCodeAt(i)
		hash = (hash << 5) - hash + chr
		hash |= 0 // Convert to 32bit integer
	}
	return hash
}
export async function eval_like(
	text: string,
	context = {},
	state: Record<string, any>,
	editor: boolean,
	controlComponents: Record<
		string,
		{
			setTab?: (index: number) => void
			agGrid?: { api: any; columnApi: any }
			setValue?: (value: any) => void
			setSelectedIndex?: (index: number) => void
			openModal?: () => void
			closeModal?: () => void
			open?: () => void
			close?: () => void
			validate?: (key: string) => void
			invalidate?: (key: string, error: string) => void
			validateAll?: () => void
			clearFiles?: () => void
			showToast?: (message: string, error?: boolean) => void
			waitJob?: (jobId: string) => void
			askNewResource?: () => void
			setGroupValue?: (key: string, value: any) => void
			sendMessage?: (message: string) => void
		}
	>,
	worldStore: World | undefined,
	runnableComponents: Record<string, { cb?: (() => void)[] }>,
	noReturn: boolean,
	groupContextId: string | undefined,
	globalRecomputeFunction: ((excludeIds?: string) => void) | undefined
) {
	const createProxy = (name: string, obj: any) => {
		// console.log('Creating proxy', name, obj)
		if (obj != null && obj != undefined && typeof obj == 'object') {
			if (name == 'group' && groupContextId) {
				return createGroupProxy(groupContextId, obj)
			}
			return new Proxy(obj, {
				set(target, key, value) {
					if (name != 'state') {
						throw new Error(
							'Cannot set value on objects that are neither the global state or a container group field'
						)
					}
					if (typeof key !== 'string') {
						throw new Error('Invalid key')
					}
					target[key] = value
					let o = worldStore?.newOutput(name, key, value)
					o?.set(value, true)

					return true
				},
				get(obj, prop) {
					if (name != 'state' && prop == 'group') {
						return createGroupProxy(name, obj[prop])
					} else {
						return obj[prop]
					}
				}
			})
		} else {
			return obj
		}
	}

	const createGroupProxy = (name: string, obj: any) => {
		return new Proxy(obj, {
			set(target, key, value) {
				target[key] = value
				let o = worldStore?.newOutput(name, 'group', target)
				o?.set(target, true)
				if (typeof key !== 'string') {
					throw new Error('Invalid key')
				}
				controlComponents[name]?.setGroupValue?.(key, value)
				return true
			}
		})
	}

	const proxiedState = createProxy('state', state)

	let evaluator = make_context_evaluator(text, Object.keys(context ?? {}), noReturn)
	// console.log(i, j)
	return await evaluator(
		context,
		proxiedState,
		createProxy,
		async (x, newTab) => {
			if (newTab || editor) {
				if (!newTab) {
					sendUserToast(
						'In editor mode, `goto` opens a new tab to prevent losing your work. To test the redirection , use the preview mode.'
					)
				}
				window.open(x, '_blank')
			} else {
				window.location.href = x
			}
		},
		(id, index) => {
			controlComponents[id]?.setTab?.(index)
		},
		(id) => {
			runnableComponents[id]?.cb?.forEach((f) => f())
		},
		() => {
			const callerId = ((context ?? {}) as any).id
			if (callerId) {
				globalRecomputeFunction?.(callerId)
			}
		},
		(id) => {
			return controlComponents[id]?.agGrid
		},
		(id, value) => {
			controlComponents[id]?.setValue?.(value)
		},
		(id, index) => {
			controlComponents[id]?.setSelectedIndex?.(index)
		},
		(id) => {
			controlComponents[id]?.openModal?.()
		},
		(id) => {
			controlComponents[id]?.closeModal?.()
		},
		(id) => {
			controlComponents[id]?.open?.()
		},
		(id) => {
			controlComponents[id]?.close?.()
		},
		(id, key) => {
			controlComponents[id]?.validate?.(key)
		},
		(id, key, error) => {
			controlComponents[id]?.invalidate?.(key, error)
		},
		(id) => {
			controlComponents[id]?.validateAll?.()
		},
		(id) => {
			controlComponents[id]?.clearFiles?.()
		},
		(id, message) => {
			controlComponents[id]?.sendMessage?.(message)
		},
		(message, error) => {
			sendUserToast(message, error)
		},
		async (id) => {
			const workspaceId = ((context ?? {}) as any)?.ctx?.workspace
			return await waitJob(id, workspaceId)
		},
		(id) => {
			controlComponents[id]?.askNewResource?.()
		},
		(input, filename) => {
			const handleError = (error) => {
				console.error('Error downloading file:', error)
				sendUserToast(
					`Error downloading file: ${error.message}. Ensure it is a valid URL, a base64 encoded data URL (data:...), or a valid S3 object.`,
					true
				)
			}

			const isBase64 = (str) => {
				try {
					return btoa(atob(str)) === str
				} catch (err) {
					return false
				}
			}

			const downloadFile = (url, downloadFilename) => {
				console.log(url, downloadFilename)
				const link = document.createElement('a')
				link.href = url
				link.download = downloadFilename || true
				link.target = '_blank'
				link.rel = 'external'
				document.body.appendChild(link)
				link.click()
				document.body.removeChild(link)
			}

			if (typeof input === 'object' && input.s3) {
				const workspaceId = ((context ?? {}) as any).ctx?.workspace
				const s3href = `${base}/api/w/${workspaceId}/job_helpers/download_s3_file?file_key=${encodeURIComponent(
					input?.s3 ?? ''
				)}${input?.storage ? `&storage=${input.storage}` : ''}`
				downloadFile(s3href, filename || input.s3)
			} else if (typeof input === 'string') {
				if (input.startsWith('data:')) {
					downloadFile(input, filename)
				} else if (isBase64(input)) {
					const base64Url = `data:application/octet-stream;base64,${input}`
					downloadFile(base64Url, filename)
				} else if (/^(http|https):\/\//.test(input) || input.startsWith('/')) {
					const url = input.startsWith('/') ? `${window.location.origin}${input}` : input
					console.log('Downloading file from:', url)
					downloadFile(url, filename ?? url.split('/').pop()?.split('?')[0])
				} else {
					handleError(
						new Error(
							'The input must be a valid URL, a base64 encoded string, or a valid S3 object.'
						)
					)
				}
			} else {
				handleError(
					new Error('The input must be a string or an object with a getAuthenticatedUrl method.')
				)
			}
		}
	)
}

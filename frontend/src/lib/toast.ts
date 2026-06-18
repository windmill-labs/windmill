import Toast from '$lib/components/Toast.svelte'
import { toast } from '@zerodevx/svelte-toast'
import type { ComponentProps } from 'svelte'
import type { Button } from './components/common'
import type { AlertType } from '$lib/components/common/alert/model'

export type ToastType = AlertType

export type ToastAction = {
	label: string
	callback: () => void
	buttonType?: ComponentProps<typeof Button>['variant']
}

export function sendUserToast(
	message: string,
	_type: boolean | ToastType = 'success',
	actions: ToastAction[] = [],
	errorMessage: string | undefined = undefined,
	duration: number = 5000
): {
	destroy: () => void
} {
	const type = typeof _type === 'boolean' ? (_type ? 'error' : 'success') : _type
	const error = type === 'error'
	if (globalThis.windmillToast) {
		globalThis.windmillToast({
			message,
			error,
			type,
			actions,
			errorMessage,
			duration
		})
		return { destroy: () => {} }
	}
	const id = toast.push({
		component: {
			// https://github.com/zerodevx/svelte-toast/issues/115
			// Svelte 5 changed its component type and svelte-toast is not up to date yet
			// @ts-ignore
			src: Toast,
			props: {
				message,
				type,
				actions,
				errorMessage,
				duration
			},
			sendIdTo: 'toastId'
		},
		dismissable: false,
		initial: 0,
		theme: {
			'--toastPadding': '0',
			'--toastMsgPadding': '0',
			'--toastBackground': '#00000000',
			'--toastBorderRadius': '0.4rem',
			'--toastWidth': '34rem',
			'--toastMinHeight': '1rem',
			'--toastBoxShadow': 'none'
		}
	})

	return {
		destroy: () => toast.pop(id)
	}
}

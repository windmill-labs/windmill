import Toast from '$lib/components/Toast.svelte'
import { toast } from '@zerodevx/svelte-toast'
import type { ComponentProps } from 'svelte'
import type { Button } from './components/common'

export type ToastAction = {
	label: string
	callback: () => void
	buttonType?: ComponentProps<typeof Button>['variant']
}

export function sendUserToast(
	message: string,
	error: boolean = false,
	actions: ToastAction[] = [],
	errorMessage: string | undefined = undefined,
	duration: number = 5000
): void {
	if (globalThis.windmillToast) {
		globalThis.windmillToast({
			message,
			error,
			actions,
			errorMessage,
			duration
		})
		return
	}
	toast.push({
		component: {
			// https://github.com/zerodevx/svelte-toast/issues/115
			// Svelte 5 changed its component type and svelte-toast is not up to date yet
			// @ts-ignore
			src: Toast,
			props: {
				message,
				error,
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
			'--toastBorderRadius': '0.4rem'
		}
	})
}

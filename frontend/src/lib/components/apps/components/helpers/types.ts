export type SideEffectAction =
	| {
			selected:
				| 'gotoUrl'
				| 'none'
				| 'setTab'
				| 'sendToast'
				| 'sendErrorToast'
				| 'errorOverlay'
				| 'openModal'
				| 'closeModal'
				| 'open'
				| 'close'
				| 'clearFiles'
			configuration: {
				gotoUrl: { url: (() => string) | string | undefined; newTab: boolean | undefined }
				setTab: {
					setTab:
						| (() => { id: string; index: number }[])
						| { id: string; index: number }[]
						| undefined
				}
				sendToast?: {
					message: (() => string) | string | undefined
				}
				sendErrorToast?: {
					message: (() => string) | string | undefined
					appendError: boolean | undefined
				}
				openModal?: {
					modalId: string | undefined
				}
				closeModal?: {
					modalId: string | undefined
				}
				open?: {
					id: string | undefined
				}
				close?: {
					id: string | undefined
				}
				clearFiles?: {
					id: string | undefined
				}
			}
	  }
	| undefined

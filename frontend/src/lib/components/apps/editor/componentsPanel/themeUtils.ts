import { ResourceService, AppService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import type { AppTheme } from '../../types'

export interface Theme {
	path: string
	value: {
		value?: string | undefined
		name: string
	}
}

export const DEFAULT_THEME: string = 'f/app_themes/theme_0'

export async function createTheme(workspace: string, theme: Theme): Promise<string> {
	const createThemeRequest = {
		workspace,
		requestBody: {
			...theme,
			resource_type: 'app_theme',
			value: theme.value || ''
		}
	}
	try {
		return await ResourceService.createResource(createThemeRequest)
	} catch (e) {
		throw Error(`Error saving theme at resource ${theme.path}, you do not have enough permission`)
	}
}

export async function getTheme(
	workspace: string,
	path: string
): Promise<{
	value?: string | undefined
	name: string
}> {
	try {
		return AppService.getPublicResource({
			workspace,
			path
		}) as any
	} catch (e) {
		sendUserToast(`Theme not found ${path}`)
		return {
			value: '',
			name: 'Not found'
		}
	}
}

export function updateTheme(workspace: string, path: string, updatedTheme: any): Promise<string> {
	const updateThemeRequest = {
		workspace,
		path,
		requestBody: updatedTheme
	}
	return ResourceService.updateResource(updateThemeRequest)
}

export function deleteTheme(workspace: string, path: string): Promise<string> {
	const deleteThemeRequest = {
		workspace,
		path: path
	}
	return ResourceService.deleteResource(deleteThemeRequest)
}

export async function listThemes(workspace: string): Promise<
	Array<{
		name: string
		path: string
	}>
> {
	const listThemesRequest = {
		workspace,
		name: 'app_theme'
	}
	return ResourceService.listResourceNames(listThemesRequest)
}

export async function resolveTheme(
	theme: AppTheme | undefined,
	workspace: string | undefined
): Promise<string> {
	let css = ''
	if (theme?.type === 'inlined') {
		css = theme.css
	} else if (theme?.type === 'path' && theme.path && workspace) {
		let loadedCss: any = { value: '' }
		try {
			loadedCss = await ResourceService.getResourceValue({
				workspace: workspace,
				path: theme.path
			})
		} catch (e) {
			console.error('Error loading theme', e)
		}

		css = (loadedCss as any)?.['value'] ?? ''
	}
	return css
}

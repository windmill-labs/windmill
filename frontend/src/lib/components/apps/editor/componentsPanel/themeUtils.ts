import { ResourceService, type Resource } from '$lib/gen'
import type { AppTheme } from '../../types'

export interface Theme {
	path: string
	value: {
		value?: string | undefined
		name: string
	}
}

export function createTheme(workspace: string, theme: Theme): Promise<string> {
	const createThemeRequest = {
		workspace,
		requestBody: {
			...theme,
			resource_type: 'theme',
			value: theme.value || ''
		}
	}
	return ResourceService.createResource(createThemeRequest)
}

export function getTheme(workspace: string, path: string): Promise<Resource> {
	const getResourceRequest = {
		workspace,
		path
	}
	return ResourceService.getResource(getResourceRequest)
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
		name: 'theme'
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
		let loadedCss = await ResourceService.getResource({
			workspace: workspace,
			path: theme.path
		})

		css = loadedCss.value.value
	}
	return css
}

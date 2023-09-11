import { ResourceService } from '$lib/gen'

export interface Theme {
	path: string
	value?: string | undefined
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

export function getTheme(workspace: string, themeName: string): Promise<Theme> {
	const getResourceRequest = {
		workspace,
		path: `themes/${themeName}`
	}
	return ResourceService.getResource(getResourceRequest)
}

export function updateTheme(
	workspace: string,
	themeName: string,
	updatedTheme: Theme
): Promise<string> {
	const updateThemeRequest = {
		workspace,
		path: `themes/${themeName}`,
		requestBody: updatedTheme
	}
	return ResourceService.updateResource(updateThemeRequest)
}

export function deleteTheme(workspace: string, themeName: string): Promise<string> {
	const deleteThemeRequest = {
		workspace,
		path: `themes/${themeName}`
	}
	return ResourceService.deleteResource(deleteThemeRequest)
}

export async function listThemes(workspace: string): Promise<Theme[]> {
	const listThemesRequest = {
		workspace,
		resourceType: 'theme'
	}
	const resources = await ResourceService.listResource(listThemesRequest)

	// use ResourceService.getResource to get the theme value for each theme

	await Promise.all(
		resources.map(async (resource) => {
			const theme = await ResourceService.getResource({ workspace, path: resource.path })
			resource.value = theme.value
		})
	)

	return resources
}

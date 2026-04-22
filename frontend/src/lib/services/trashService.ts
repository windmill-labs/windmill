import { OpenAPI } from '$lib/gen/core/OpenAPI'
import { request as __request } from '$lib/gen/core/request'

export type TrashItem = {
	id: number
	workspace_id: string
	item_kind: string
	item_path: string
	deleted_by: string
	deleted_at: string
	expires_at: string
}

export class TrashService {
	public static listTrash(data: {
		workspace: string
		itemKind?: string
		page?: number
		perPage?: number
	}): Promise<TrashItem[]> {
		return __request(OpenAPI, {
			method: 'GET',
			url: '/w/{workspace}/trash/list',
			path: {
				workspace: data.workspace
			},
			query: {
				item_kind: data.itemKind,
				page: data.page,
				per_page: data.perPage
			}
		})
	}

	public static restoreTrashItem(data: { workspace: string; id: number }): Promise<string> {
		return __request(OpenAPI, {
			method: 'POST',
			url: '/w/{workspace}/trash/restore/{id}',
			path: {
				workspace: data.workspace,
				id: data.id
			}
		})
	}

	public static permanentlyDeleteTrashItem(data: {
		workspace: string
		id: number
	}): Promise<string> {
		return __request(OpenAPI, {
			method: 'DELETE',
			url: '/w/{workspace}/trash/delete/{id}',
			path: {
				workspace: data.workspace,
				id: data.id
			}
		})
	}

	public static emptyTrash(data: { workspace: string }): Promise<string> {
		return __request(OpenAPI, {
			method: 'POST',
			url: '/w/{workspace}/trash/empty',
			path: {
				workspace: data.workspace
			}
		})
	}
}

import { error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { APP_TO_ICON_COMPONENT } from '$lib/components/icons';
 
export const GET = (({ params }) => {
  const { app } = params;
	const icon = APP_TO_ICON_COMPONENT[app];

	if(!icon) {
		throw error(404, 'Not found');
	}
 
	const render = icon.render().html;
  return new Response(render, {
		headers: {
			'Content-Type': 'image/svg+xml'
		}
	});
}) satisfies RequestHandler;
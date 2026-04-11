import type { Handle, HandleServerError } from '@sveltejs/kit';

export const handle: Handle = async ({ event, resolve }) => {
	const response = await resolve(event, {
		// `js` is preloaded as modulepreload by SvelteKit by default; we only need
		// to opt fonts and CSS in explicitly.
		preload: ({ type }) => type === 'font' || type === 'css'
	});

	// Security headers
	response.headers.set('X-Frame-Options', 'SAMEORIGIN');
	response.headers.set('X-Content-Type-Options', 'nosniff');
	response.headers.set('Referrer-Policy', 'strict-origin-when-cross-origin');
	response.headers.set(
		'Permissions-Policy',
		'camera=(), microphone=(), geolocation=(), interest-cohort=()'
	);

	// Cache immutable assets (Vite hashed files)
	const { pathname } = event.url;
	if (pathname.startsWith('/_app/immutable/')) {
		response.headers.set('Cache-Control', 'public, max-age=31536000, immutable');
	}

	return response;
};

export const handleError: HandleServerError = ({ error, event, status, message }) => {
	const errorId = crypto.randomUUID();
	console.error(`[server-error ${errorId}]`, status, message, event.url.pathname, error);
	return {
		message: status >= 500 ? 'An unexpected error occurred. Please try again.' : message,
		id: errorId
	};
};

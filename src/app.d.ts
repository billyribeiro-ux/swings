// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		interface Error {
			message: string;
			code?: string;
			id?: string;
		}

		interface Locals {
			user?: {
				id: string;
				email: string;
				name: string;
				role: 'member' | 'admin';
			};
		}

		// PageData is augmented per-route via `+page.ts` / `+page.server.ts` load
		// return types — leave the shared shape empty so route-specific data flows
		// through the generated `PageProps` types.
		// eslint-disable-next-line @typescript-eslint/no-empty-object-type
		interface PageData {}

		// eslint-disable-next-line @typescript-eslint/no-empty-object-type
		interface PageState {}

		// eslint-disable-next-line @typescript-eslint/no-empty-object-type
		interface Platform {}
	}
}

export {};

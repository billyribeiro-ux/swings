// Svelte 5 reactive auth state class
import { browser } from '$app/environment';

export interface AuthUser {
	id: string;
	email: string;
	name: string;
	role: 'member' | 'admin';
	avatar_url: string | null;
	created_at: string;
}

const TOKEN_KEY = 'swings_access_token';
const REFRESH_KEY = 'swings_refresh_token';
const USER_KEY = 'swings_user';

class AuthState {
	user = $state<AuthUser | null>(null);
	accessToken = $state<string | null>(null);
	refreshToken = $state<string | null>(null);
	loading = $state(true);

	isAuthenticated = $derived(!!this.user && !!this.accessToken);
	isAdmin = $derived(this.user?.role?.toLowerCase() === 'admin');
	isMember = $derived(this.user?.role?.toLowerCase() === 'member');

	constructor() {
		if (browser) {
			this.accessToken = localStorage.getItem(TOKEN_KEY);
			this.refreshToken = localStorage.getItem(REFRESH_KEY);
			const stored = localStorage.getItem(USER_KEY);
			if (stored) {
				try {
					this.user = JSON.parse(stored);
				} catch {
					this.user = null;
				}
			}
		}
		// Resolve loading on both server and client so SSR-rendered UI doesn't get
		// stuck in a "loading" state forever.
		this.loading = false;
	}

	setAuth = (user: AuthUser, accessToken: string, refreshToken: string) => {
		this.user = user;
		this.accessToken = accessToken;
		this.refreshToken = refreshToken;

		if (browser) {
			localStorage.setItem(TOKEN_KEY, accessToken);
			localStorage.setItem(REFRESH_KEY, refreshToken);
			localStorage.setItem(USER_KEY, JSON.stringify(user));
		}
	};

	setTokens = (accessToken: string, refreshToken: string) => {
		this.accessToken = accessToken;
		this.refreshToken = refreshToken;

		if (browser) {
			localStorage.setItem(TOKEN_KEY, accessToken);
			localStorage.setItem(REFRESH_KEY, refreshToken);
		}
	};

	setUser = (user: AuthUser) => {
		this.user = user;
		if (browser) {
			localStorage.setItem(USER_KEY, JSON.stringify(user));
		}
	};

	logout = () => {
		this.user = null;
		this.accessToken = null;
		this.refreshToken = null;

		if (browser) {
			localStorage.removeItem(TOKEN_KEY);
			localStorage.removeItem(REFRESH_KEY);
			localStorage.removeItem(USER_KEY);
		}
	};
}

export const auth = new AuthState();

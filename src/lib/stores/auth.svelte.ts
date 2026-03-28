// Svelte 5 reactive auth state class
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
	isAdmin = $derived(this.user?.role === 'admin');
	isMember = $derived(this.user?.role === 'member');

	constructor() {
		if (typeof window !== 'undefined') {
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
			this.loading = false;
		}
	}

	setAuth = (user: AuthUser, accessToken: string, refreshToken: string) => {
		this.user = user;
		this.accessToken = accessToken;
		this.refreshToken = refreshToken;

		if (typeof window !== 'undefined') {
			localStorage.setItem(TOKEN_KEY, accessToken);
			localStorage.setItem(REFRESH_KEY, refreshToken);
			localStorage.setItem(USER_KEY, JSON.stringify(user));
		}
	};

	setTokens = (accessToken: string, refreshToken: string) => {
		this.accessToken = accessToken;
		this.refreshToken = refreshToken;

		if (typeof window !== 'undefined') {
			localStorage.setItem(TOKEN_KEY, accessToken);
			localStorage.setItem(REFRESH_KEY, refreshToken);
		}
	};

	setUser = (user: AuthUser) => {
		this.user = user;
		if (typeof window !== 'undefined') {
			localStorage.setItem(USER_KEY, JSON.stringify(user));
		}
	};

	logout = () => {
		this.user = null;
		this.accessToken = null;
		this.refreshToken = null;

		if (typeof window !== 'undefined') {
			localStorage.removeItem(TOKEN_KEY);
			localStorage.removeItem(REFRESH_KEY);
			localStorage.removeItem(USER_KEY);
		}
	};
}

export const auth = new AuthState();

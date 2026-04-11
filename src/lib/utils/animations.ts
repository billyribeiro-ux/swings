import { gsap } from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';
import type { TransitionConfig } from 'svelte/transition';
import type { Attachment } from 'svelte/attachments';
import { cubicOut, quintOut, expoOut } from 'svelte/easing';
import { prefersReducedMotion } from 'svelte/motion';

// ---------------------------------------------------------------------------
// Register GSAP plugins once
// ---------------------------------------------------------------------------
gsap.registerPlugin(ScrollTrigger);

// ---------------------------------------------------------------------------
// Reduced-motion helper (reads Svelte 5's reactive signal at call time)
// ---------------------------------------------------------------------------
export function isReducedMotion(): boolean {
	return prefersReducedMotion.current;
}

// ---------------------------------------------------------------------------
// GSAP cinematic presets
// ---------------------------------------------------------------------------
export const EASE = {
	/** Snappy deceleration -- buttons, badges */
	snappy: 'power4.out',
	/** Smooth cinematic -- headings, hero text */
	cinematic: 'expo.out',
	/** Soft float -- subtitles, descriptions */
	soft: 'power2.out',
	/** Elastic bounce -- icons, accents */
	elastic: 'elastic.out(1, 0.5)',
	/** Breathing loop -- glow orbs */
	breathe: 'sine.inOut',
	/** Back overshoot -- cards popping in */
	back: 'back.out(1.4)'
} as const;

export const DURATION = {
	fast: 0.5,
	normal: 0.7,
	slow: 0.9,
	cinematic: 1.2,
	breathe: 7
} as const;

// ---------------------------------------------------------------------------
// Reusable GSAP timeline builder for hero cascades
// ---------------------------------------------------------------------------
export interface CascadeItem {
	selector: string;
	duration?: number;
	ease?: string;
	y?: number;
	blur?: number;
	scale?: number;
	overlap?: number; // negative offset from previous
}

export function createCinematicCascade(
	scope: HTMLElement,
	items: CascadeItem[],
	opts?: { delay?: number }
): gsap.core.Timeline {
	const reduced = isReducedMotion();
	const tl = gsap.timeline({ delay: opts?.delay ?? 0.2 });

	items.forEach((item, i) => {
		const dur = reduced ? 0.01 : (item.duration ?? DURATION.normal);
		const ease = reduced ? 'none' : (item.ease ?? EASE.cinematic);
		const y = reduced ? 0 : (item.y ?? 30);
		const blurAmt = reduced ? 0 : (item.blur ?? 8);
		const scaleAmt = reduced ? 1 : (item.scale ?? 0.97);
		const overlap = reduced ? 0 : (item.overlap ?? 0.55);

		// Initial state
		gsap.set(item.selector, {
			opacity: 0,
			y,
			scale: scaleAmt,
			filter: `blur(${blurAmt}px)`,
			willChange: 'transform, opacity, filter'
		});

		const offset = i === 0 ? undefined : `-=${overlap}`;

		tl.to(
			item.selector,
			{
				opacity: 1,
				y: 0,
				scale: 1,
				filter: 'blur(0px)',
				duration: dur,
				ease
			},
			offset
		);
	});

	// Cleanup willChange after cascade completes
	tl.call(() => {
		items.forEach((item) => {
			gsap.set(item.selector, {
				willChange: 'auto',
				clearProps: 'filter'
			});
		});
	});

	return tl;
}

// ---------------------------------------------------------------------------
// Reusable GSAP scroll-reveal with cinematic motion
// ---------------------------------------------------------------------------
export interface CinematicRevealOpts {
	targets: Element[] | NodeListOf<Element> | HTMLCollection;
	trigger: Element;
	y?: number;
	blur?: number;
	scale?: number;
	duration?: number;
	stagger?: number;
	ease?: string;
	start?: string;
}

export function createCinematicReveal(opts: CinematicRevealOpts): gsap.core.Tween {
	const reduced = isReducedMotion();

	gsap.set(opts.targets, {
		opacity: 0,
		y: reduced ? 0 : (opts.y ?? 40),
		scale: reduced ? 1 : (opts.scale ?? 0.96),
		filter: reduced ? 'none' : `blur(${opts.blur ?? 6}px)`,
		willChange: 'transform, opacity, filter'
	});

	return gsap.to(opts.targets, {
		opacity: 1,
		y: 0,
		scale: 1,
		filter: 'blur(0px)',
		duration: reduced ? 0.01 : (opts.duration ?? DURATION.slow),
		stagger: reduced ? 0 : (opts.stagger ?? 0.12),
		ease: reduced ? 'none' : (opts.ease ?? EASE.cinematic),
		scrollTrigger: {
			trigger: opts.trigger,
			start: opts.start ?? 'top 82%',
			once: true
		},
		onComplete() {
			gsap.set(opts.targets, {
				willChange: 'auto',
				clearProps: 'filter,transform'
			});
		}
	});
}

// ---------------------------------------------------------------------------
// Glow orb ambient breathing animation
// ---------------------------------------------------------------------------
export function createGlowBreathing(
	element: HTMLElement,
	opts?: { scale?: number; opacity?: number; duration?: number }
): gsap.core.Tween {
	if (isReducedMotion()) {
		return gsap.to(element, { opacity: opts?.opacity ?? 0.5, duration: 0.01 });
	}

	return gsap.to(element, {
		scale: opts?.scale ?? 1.12,
		opacity: opts?.opacity ?? 0.65,
		duration: opts?.duration ?? DURATION.breathe,
		ease: EASE.breathe,
		yoyo: true,
		repeat: -1
	});
}

// ---------------------------------------------------------------------------
// Custom Svelte transitions -- cinematic quality
// ---------------------------------------------------------------------------

/** Cinematic blur-fade: opacity + blur + y-translate + scale */
export function cinematicFade(
	_node: Element,
	{
		delay = 0,
		duration = 600,
		y = 30,
		blur: blurAmt = 8,
		scaleFrom = 0.97,
		easing = quintOut
	}: {
		delay?: number;
		duration?: number;
		y?: number;
		blur?: number;
		scaleFrom?: number;
		easing?: (t: number) => number;
	} = {}
): TransitionConfig {
	if (isReducedMotion()) {
		return { delay, duration: 150, easing: cubicOut, css: (t) => `opacity: ${t}` };
	}

	return {
		delay,
		duration,
		easing,
		css: (t) => {
			const currentBlur = blurAmt * (1 - t);
			const currentY = y * (1 - t);
			const currentScale = scaleFrom + (1 - scaleFrom) * t;
			return `
				opacity: ${t};
				transform: translateY(${currentY}px) scale(${currentScale});
				filter: blur(${currentBlur}px);
			`;
		}
	};
}

/** Clip-path wipe reveal -- slides a rectangular mask to reveal content */
export function clipReveal(
	_node: Element,
	{
		delay = 0,
		duration = 800,
		direction = 'up',
		easing = expoOut
	}: {
		delay?: number;
		duration?: number;
		direction?: 'up' | 'down' | 'left' | 'right';
		easing?: (t: number) => number;
	} = {}
): TransitionConfig {
	if (isReducedMotion()) {
		return { delay, duration: 150, easing: cubicOut, css: (t) => `opacity: ${t}` };
	}

	return {
		delay,
		duration,
		easing,
		css: (t) => {
			const u = 1 - t;
			let clip: string;
			switch (direction) {
				case 'up':
					clip = `inset(${u * 100}% 0 0 0)`;
					break;
				case 'down':
					clip = `inset(0 0 ${u * 100}% 0)`;
					break;
				case 'left':
					clip = `inset(0 ${u * 100}% 0 0)`;
					break;
				case 'right':
					clip = `inset(0 0 0 ${u * 100}%)`;
					break;
			}
			return `clip-path: ${clip}; opacity: ${Math.min(t * 1.5, 1)};`;
		}
	};
}

// ---------------------------------------------------------------------------
// Svelte 5.29+ attachment factories — modern replacement for Svelte actions.
// Use as `<div {@attach cinematicReveal({ y: 40, blur: 6 })}>...</div>`. The
// attachment runs once when the element is mounted, has access to the node,
// and the returned cleanup is called automatically on unmount.
// ---------------------------------------------------------------------------

export interface CinematicRevealAttachOpts {
	y?: number;
	blur?: number;
	scale?: number;
	duration?: number;
	stagger?: number;
	ease?: string;
	start?: string;
	/** Optional CSS selector to animate within the host node; defaults to direct children */
	selector?: string;
}

/** Attachment factory: wraps `createCinematicReveal` so any element can opt into the cinematic GSAP reveal without `bind:this` + `onMount` boilerplate. */
export function cinematicReveal(opts: CinematicRevealAttachOpts = {}): Attachment<HTMLElement> {
	return (node) => {
		const targets = opts.selector ? node.querySelectorAll(opts.selector) : node.children;
		if (!targets.length) return;

		const ctx = gsap.context(() => {
			createCinematicReveal({
				targets,
				trigger: node,
				y: opts.y,
				blur: opts.blur,
				scale: opts.scale,
				duration: opts.duration,
				stagger: opts.stagger,
				ease: opts.ease,
				start: opts.start
			});
		}, node);

		return () => {
			ctx.revert();
			if (!isReducedMotion()) gsap.set(targets, { clearProps: 'all' });
		};
	};
}

/** Attachment factory: wraps `createGlowBreathing` for ambient glow orbs. */
export function glowBreathing(opts?: {
	scale?: number;
	opacity?: number;
	duration?: number;
}): Attachment<HTMLElement> {
	return (node) => {
		const tween = createGlowBreathing(node, opts);
		return () => tween.kill();
	};
}

/** Scale + blur pop -- great for icons, badges, avatars */
export function popIn(
	_node: Element,
	{
		delay = 0,
		duration = 500,
		scaleFrom = 0.7,
		blur: blurAmt = 10,
		easing = quintOut
	}: {
		delay?: number;
		duration?: number;
		scaleFrom?: number;
		blur?: number;
		easing?: (t: number) => number;
	} = {}
): TransitionConfig {
	if (isReducedMotion()) {
		return { delay, duration: 150, easing: cubicOut, css: (t) => `opacity: ${t}` };
	}

	return {
		delay,
		duration,
		easing,
		css: (t) => {
			const currentScale = scaleFrom + (1 - scaleFrom) * t;
			const currentBlur = blurAmt * (1 - t);
			return `
				opacity: ${t};
				transform: scale(${currentScale});
				filter: blur(${currentBlur}px);
			`;
		}
	};
}

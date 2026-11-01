<!--
  Embeds the Cookie D'oh! wasm build in a SvelteKit page (Svelte 5).

  Copy the release build into your app's static folder first:
    task web:export DEST=../my-site/static/cookie-doh

  Then:
    <CookieDoh onexit={() => goto('/games')} ongameover={(r) => saveScore(r.score)} />

  The game binds to its <canvas> once and has no teardown, so the canvas and the wasm module
  live for the whole page session. Unmounting this component only detaches the canvas (the
  game pauses and goes silent while detached); mounting it again re-attaches the same canvas
  and resumes, or, with `fresh`, starts over from the title (or cracktro) with the current
  `cracktro` and `gfx` props. Starting over keeps the sound on once the player has unlocked it.

  `restart()` (exported from this module) starts over at any time.
-->
<script module lang="ts">
	export interface GameOver {
		score: number;
		level: number;
		hiscore: number;
		/** The score beat the stored hi-score. */
		record: boolean;
	}

	let canvas: HTMLCanvasElement | null = null;
	let started: Promise<void> | null = null;

	function getCanvas(cracktro: boolean, gfx: string | undefined): HTMLCanvasElement {
		if (!canvas) {
			canvas = document.createElement('canvas');
			canvas.id = 'cookie-doh';
			canvas.tabIndex = 0;
			canvas.setAttribute('aria-label', "Cookie D'oh! arcade game");
			if (cracktro) canvas.dataset.cracktro = '';
			if (gfx) canvas.dataset.gfx = gfx;
		}
		return canvas;
	}

	/** Start over as if freshly launched; omitted options fall back to the launch options. */
	export function restart(options: { cracktro?: boolean; gfx?: string } = {}): void {
		canvas?.dispatchEvent(new CustomEvent('cookie-doh:restart', { detail: options }));
	}

	// `ref.null func; drop`: valid only with WebAssembly reference types, which the build uses
	// (like every Rust wasm build since 1.82). There is no polyfill for missing wasm features.
	const REF_TYPES = new Uint8Array([
		0, 97, 115, 109, 1, 0, 0, 0, 1, 4, 1, 96, 0, 0, 3, 2, 1, 0, 10, 7, 1, 5, 0, 208, 112, 26, 11
	]);

	/** Why this browser can't run the game, or `null` if it can. */
	function unsupported(): string | null {
		if (typeof WebAssembly !== 'object' || !WebAssembly.validate(REF_TYPES)) {
			return 'This browser is too old for the game. It runs in Chrome 96, Firefox 79, Safari 15 or newer.';
		}
		if (!('gpu' in navigator)) {
			const gl = document.createElement('canvas').getContext('webgl2');
			if (!gl) return 'The game needs WebGPU or WebGL2, which this browser has turned off or lacks.';
			gl.getExtension('WEBGL_lose_context')?.loseContext();
		}
		return null;
	}

	function start(base: string): Promise<void> {
		started ??= import(/* @vite-ignore */ `${base}/cookie-doh.js`).then((m) => m.default());
		return started;
	}
</script>

<script lang="ts">
	interface Props {
		/** URL path the build was exported to (no trailing slash). */
		base?: string;
		/** Focus the game on mount so the keyboard works right away. */
		autofocus?: boolean;
		/** Show the Cookies & Crackers cracktro first. */
		cracktro?: boolean;
		/** Start in a historical graphics mode: 'ega' | 'c64' | 'cga' | 'hercules'. */
		gfx?: 'vga' | 'ega' | 'c64' | 'cga' | 'hercules';
		/** Start over when mounted again, instead of resuming where the player left. */
		fresh?: boolean;
		/** The player pressed Esc on the title screen; the game has gone quiet. */
		onexit?: () => void;
		/** A game started. */
		onstart?: () => void;
		/** A game ended. */
		ongameover?: (result: GameOver) => void;
	}

	let {
		base = '/cookie-doh',
		autofocus = false,
		cracktro = false,
		gfx,
		fresh = false,
		onexit,
		onstart,
		ongameover
	}: Props = $props();

	let error = $state<string | null>(null);

	/** Attach the shared canvas to `slot`, start the game once, detach on unmount. */
	function mount(slot: HTMLElement) {
		const why = unsupported();
		if (why) {
			error = why;
			return;
		}
		const c = getCanvas(cracktro, gfx);
		const exit = () => onexit?.();
		const begin = () => onstart?.();
		const over = (e: Event) => ongameover?.((e as CustomEvent<GameOver>).detail);
		c.addEventListener('cookie-doh:exit', exit);
		c.addEventListener('cookie-doh:start', begin);
		c.addEventListener('cookie-doh:gameover', over);
		const again = started !== null;
		slot.appendChild(c);
		if (autofocus) c.focus();
		if (again && fresh) restart({ cracktro, gfx: gfx ?? 'vga' });
		// The canvas must be in the document before the game starts, as it looks it up by id.
		start(base).catch((e) => {
			// winit throws a control-flow exception once the event loop is running; that is expected.
			if (!String(e).includes('Using exceptions for control flow')) {
				error = `Could not start the game: ${e}`;
				started = null;
			}
		});
		return () => {
			c.removeEventListener('cookie-doh:exit', exit);
			c.removeEventListener('cookie-doh:start', begin);
			c.removeEventListener('cookie-doh:gameover', over);
			c.remove();
		};
	}
</script>

<div class="cookie-doh">
	<div class="slot" {@attach mount}></div>
	{#if error}
		<p class="error">{error}</p>
	{/if}
</div>

<style>
	.cookie-doh {
		position: relative;
		width: 100%;
		max-width: 1280px;
		/* 320x200: the shape of the game's screen; it letterboxes inside. */
		aspect-ratio: 16 / 10;
		background: #000;
	}
	.slot {
		position: absolute;
		inset: 0;
	}
	.slot :global(canvas#cookie-doh) {
		display: block;
		/* winit sets an inline size; the container decides instead. */
		width: 100% !important;
		height: 100% !important;
		outline: none;
		touch-action: none;
		user-select: none;
		-webkit-user-select: none;
		-webkit-touch-callout: none;
		image-rendering: pixelated;
	}
	.error {
		position: absolute;
		inset: 0;
		margin: 0;
		display: grid;
		place-items: center;
		color: #f06a6a;
		font: 14px system-ui, sans-serif;
	}
</style>

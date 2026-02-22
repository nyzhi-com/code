export default {
	async fetch(request: Request): Promise<Response> {
		const url = new URL(request.url);
		if (url.pathname !== '/hero.svg') {
			return new Response('not found', { status: 404 });
		}

		const W = 720;
		const H = 380;
		const cx = W / 2;
		const copper = '#C49A6C';
		const copperDim = 'rgba(196,154,108,0.15)';
		const copperFaint = 'rgba(196,154,108,0.08)';
		const borderSub = 'rgba(255,255,255,0.07)';
		const borderDef = 'rgba(255,255,255,0.10)';

		const stats: [string, string][] = [
			['50+', 'tools'],
			['6', 'crates'],
			['0', 'runtime deps'],
		];
		const statW = 140;
		const statGap = 24;
		const statsTotal = stats.length * statW + (stats.length - 1) * statGap;
		const statsX0 = (W - statsTotal) / 2;

		let statCards = '';
		stats.forEach(([num, label], i) => {
			const x = statsX0 + i * (statW + statGap);
			const y = 200;
			statCards += `<rect x="${x}" y="${y}" width="${statW}" height="60" rx="8" fill="#0A0A0A" stroke="${borderDef}" stroke-width="1"/>`;
			statCards += `<text x="${x + statW / 2}" y="${y + 28}" class="sans" fill="${copper}" font-size="20" font-weight="600" text-anchor="middle">${num}</text>`;
			statCards += `<text x="${x + statW / 2}" y="${y + 48}" class="mono" fill="#71717A" font-size="11" text-anchor="middle">${label}</text>`;
		});

		const svg = `<svg width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg">
<defs>
  <style>
    .sans { font-family: -apple-system, 'Segoe UI', Helvetica, Arial, sans-serif; }
    .mono { font-family: ui-monospace, 'Cascadia Code', Menlo, monospace; }
  </style>
  <radialGradient id="glow" cx="50%" cy="30%" r="60%">
    <stop offset="0%" stop-color="${copperFaint}"/>
    <stop offset="100%" stop-color="transparent"/>
  </radialGradient>
  <filter id="soft" x="-50%" y="-50%" width="200%" height="200%">
    <feGaussianBlur stdDeviation="2"/>
  </filter>
</defs>

<!-- Background -->
<rect width="${W}" height="${H}" fill="#000"/>
<ellipse cx="${cx}" cy="${H * 0.3}" rx="500" ry="260" fill="url(#glow)"/>

<!-- Triskelion mark -->
<g transform="translate(${cx}, 70) scale(0.22)">
  <circle r="26" fill="${copper}"/>
  <g stroke="${copper}" stroke-width="16" fill="none" stroke-linecap="round">
    <path d="M 0,0 C 8,-42 58,-85 22,-155"/>
    <path d="M 0,0 C 8,-42 58,-85 22,-155" transform="rotate(120)"/>
    <path d="M 0,0 C 8,-42 58,-85 22,-155" transform="rotate(240)"/>
  </g>
  <circle cx="22" cy="-155" r="21" fill="${copper}"/>
  <circle cx="22" cy="-155" r="21" fill="${copper}" transform="rotate(120)"/>
  <circle cx="22" cy="-155" r="21" fill="${copper}" transform="rotate(240)"/>
  <animateTransform attributeName="transform" type="rotate" from="0" to="360" dur="60s" repeatCount="indefinite" additive="sum"/>
</g>

<!-- Copper glow behind mark -->
<circle cx="${cx}" cy="70" r="30" fill="${copper}" opacity="0.06" filter="url(#soft)">
  <animate attributeName="opacity" values="0.04;0.1;0.04" dur="4s" repeatCount="indefinite"/>
</circle>

<!-- Wordmark -->
<text x="${cx}" y="130" class="sans" fill="#FAFAFA" font-size="28" font-weight="600" text-anchor="middle" letter-spacing="-1">nyzhi</text>

<!-- Tagline -->
<text x="${cx}" y="160" class="sans" fill="#A1A1AA" font-size="14" text-anchor="middle">Single binary. No runtime deps. Ships 50+ tools.</text>

<!-- Separator -->
<line x1="${W * 0.25}" y1="180" x2="${W * 0.75}" y2="180" stroke="${copperDim}" stroke-width="1"/>

<!-- Stat cards -->
${statCards}

<!-- Install command -->
<rect x="${(W - 420) / 2}" y="290" width="420" height="36" rx="8" fill="#0A0A0A" stroke="${borderSub}" stroke-width="1"/>
<text x="${cx - 4}" y="313" class="mono" fill="#71717A" font-size="12" text-anchor="middle">$ curl -fsSL https://get.nyzhi.com | sh</text>

<!-- Blinking cursor -->
<rect x="${cx + 170}" y="303" width="7" height="14" fill="${copper}">
  <animate attributeName="opacity" values="1;0;1" dur="1.2s" repeatCount="indefinite"/>
</rect>

<!-- Subtle bottom border -->
<line x1="0" y1="${H - 1}" x2="${W}" y2="${H - 1}" stroke="${copperDim}" stroke-width="1"/>

</svg>`;

		return new Response(svg.trim(), {
			headers: {
				'Content-Type': 'image/svg+xml',
				'Cache-Control': 'no-cache, no-store, must-revalidate',
				'Access-Control-Allow-Origin': '*',
			},
		});
	},
};

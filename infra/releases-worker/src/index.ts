interface Env {
  RELEASES: R2Bucket;
}

const VALID_OS = ["darwin", "linux"] as const;
const VALID_ARCH = ["x86_64", "aarch64"] as const;

const CORS_HEADERS: HeadersInit = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, OPTIONS",
};

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (request.method === "OPTIONS") {
      return new Response(null, { status: 204, headers: CORS_HEADERS });
    }

    if (request.method !== "GET") {
      return json({ error: "Method not allowed" }, 405);
    }

    const path = url.pathname;

    if (path === "/" || path === "/install.sh") {
      return serveInstallScript(env);
    }

    if (path === "/version") {
      return serveVersion(env);
    }

    const downloadMatch = path.match(/^\/download\/([^/]+)\/([^/]+)$/);
    if (downloadMatch) {
      const [, os, arch] = downloadMatch;
      return serveDownload(env, os, arch, url.searchParams.get("version"));
    }

    return json({ error: "Not found" }, 404);
  },
} satisfies ExportedHandler<Env>;

async function serveInstallScript(env: Env): Promise<Response> {
  const obj = await env.RELEASES.get("install.sh");
  if (!obj) {
    return json({ error: "Install script not yet uploaded" }, 503);
  }
  return new Response(obj.body, {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "public, max-age=300",
      ...CORS_HEADERS,
    },
  });
}

async function serveVersion(env: Env): Promise<Response> {
  const obj = await env.RELEASES.get("releases/latest.json");
  if (!obj) {
    return json({ error: "No releases published yet" }, 503);
  }
  const data = await obj.text();
  return new Response(data, {
    headers: {
      "Content-Type": "application/json",
      "Cache-Control": "public, max-age=60",
      ...CORS_HEADERS,
    },
  });
}

async function serveDownload(
  env: Env,
  os: string,
  arch: string,
  version: string | null,
): Promise<Response> {
  if (!VALID_OS.includes(os as any)) {
    return json({ error: `Unsupported OS: ${os}. Valid: ${VALID_OS.join(", ")}` }, 400);
  }
  if (!VALID_ARCH.includes(arch as any)) {
    return json({ error: `Unsupported arch: ${arch}. Valid: ${VALID_ARCH.join(", ")}` }, 400);
  }

  let targetVersion = version;
  if (!targetVersion) {
    const latestObj = await env.RELEASES.get("releases/latest.json");
    if (!latestObj) {
      return json({ error: "No releases published yet" }, 503);
    }
    const latest = await latestObj.json<{ version: string }>();
    targetVersion = latest.version;
  }

  const key = `releases/v${targetVersion}/nyzhi-${os}-${arch}.tar.gz`;
  const obj = await env.RELEASES.get(key);
  if (!obj) {
    return json({ error: `Release not found: ${key}` }, 404);
  }

  return new Response(obj.body, {
    headers: {
      "Content-Type": "application/gzip",
      "Content-Disposition": `attachment; filename="nyzhi-${os}-${arch}.tar.gz"`,
      "Cache-Control": "public, max-age=86400, immutable",
      ...CORS_HEADERS,
    },
  });
}

function json(data: unknown, status: number = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: {
      "Content-Type": "application/json",
      ...CORS_HEADERS,
    },
  });
}

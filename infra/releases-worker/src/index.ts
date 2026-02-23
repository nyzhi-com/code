interface Env {
  RELEASES: R2Bucket;
}

type ValidOS = (typeof VALID_OS)[number];
type ValidArch = (typeof VALID_ARCH)[number];

const VALID_OS = ["darwin", "linux"] as const;
const VALID_ARCH = ["x86_64", "aarch64"] as const;

const SEMVER_RE = /^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?$/;

const CORS_HEADERS: HeadersInit = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, OPTIONS",
};

function isValidOS(s: string): s is ValidOS {
  return (VALID_OS as readonly string[]).includes(s);
}

function isValidArch(s: string): s is ValidArch {
  return (VALID_ARCH as readonly string[]).includes(s);
}

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
  if (!isValidOS(os)) {
    return json(
      { error: `Unsupported OS. Valid options: ${VALID_OS.join(", ")}` },
      400,
    );
  }
  if (!isValidArch(arch)) {
    return json(
      { error: `Unsupported architecture. Valid options: ${VALID_ARCH.join(", ")}` },
      400,
    );
  }

  let targetVersion = version;
  if (targetVersion) {
    if (!SEMVER_RE.test(targetVersion)) {
      return json({ error: "Invalid version format" }, 400);
    }
  } else {
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
    return json({ error: "Release not found" }, 404);
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

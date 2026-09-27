const ORGION_API_ORIGIN = process.env.ORGION_API_ORIGIN ?? "http://localhost:3030";

/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  agentRules: false,

  // Proxies /api/* (including the /api/ws WebSocket upgrade) through this
  // Next.js server to the real orgion-server backend, so the browser
  // only ever talks to one origin. This matters beyond convenience: the
  // session cookie is SameSite=Lax, which browsers withhold from
  // cross-site `fetch`/WebSocket calls (unlike top-level navigations) —
  // without this same-origin proxy, login would appear to succeed but
  // every subsequent request would look unauthenticated. Set
  // ORGION_API_ORIGIN if orgion-server isn't on localhost:3030.
  async rewrites() {
    return [{ source: "/api/:path*", destination: `${ORGION_API_ORIGIN}/api/:path*` }];
  },
};

export default nextConfig;

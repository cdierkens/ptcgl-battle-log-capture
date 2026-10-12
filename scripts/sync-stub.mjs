// Local stand-in for a sync destination. Paste the printed config into Settings → Sync in a dev build.
// STUB_STATUS forces a response status, e.g. STUB_STATUS=401 pnpm sync-stub.
import { createServer } from "node:http";

const port = Number(process.env.PORT ?? 8787);
const status = Number(process.env.STUB_STATUS ?? 201);

const config = {
  auth: { token: "dev-token", type: "bearer" },
  name: "Local stub",
  ptcglTrackerDestination: 1,
  url: `http://localhost:${port}/battle-logs`,
};

createServer((request, response) => {
  let body = "";
  request.on("data", (chunk) => (body += chunk));
  request.on("end", () => {
    let summary = `${body.length} bytes`;
    try {
      const envelope = JSON.parse(body);
      summary = `schema ${envelope.schemaVersion}, player ${envelope.playerName ?? "?"}, saved ${envelope.savedAt}, app ${envelope.appVersion}, ${envelope.log?.length ?? 0} chars`;
    } catch {
      // not JSON; show the size only
    }
    console.log(`${request.method} ${request.url} auth=${request.headers.authorization ?? "none"} → ${status} (${summary})`);
    response.writeHead(status, { "Content-Type": "application/json" });
    response.end(status < 300 ? JSON.stringify({ ok: true }) : JSON.stringify({ error: "stub" }));
  });
}).listen(port, () => {
  console.log(`Sync stub on http://localhost:${port}, answering ${status}. Paste this config:\n`);
  console.log(JSON.stringify(config));
});

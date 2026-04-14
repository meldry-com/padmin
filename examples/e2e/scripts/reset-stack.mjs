#!/usr/bin/env node
// Stack reset helper. Commands:
//   reset-stack.mjs         -> down -v + up -d + wait healthy (full reset)
//   reset-stack.mjs up      -> up -d + wait healthy (no data wipe)
//   reset-stack.mjs down    -> down -v (wipe volumes)
//
// Run from anywhere; the script cd's into the examples/ directory itself.

import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import http from "node:http";

const __dirname = dirname(fileURLToPath(import.meta.url));
// Run compose from the e2e/ directory so the override's relative path
// (./fixtures/pasion.test.yaml) resolves correctly. The base file is at
// ../compose.yml.
const COMPOSE_CWD = resolve(__dirname, "..");
const COMPOSE_FILES = ["-f", "../compose.yml", "-f", "fixtures/compose.override.yml"];

const mode = process.argv[2] ?? "reset";

function run(cmd, args, opts = {}) {
  const r = spawnSync(cmd, args, {
    cwd: COMPOSE_CWD,
    stdio: "inherit",
    shell: false,
    ...opts,
  });
  if (r.status !== 0) {
    throw new Error(`${cmd} ${args.join(" ")} exited ${r.status}`);
  }
}

function probe(url) {
  return new Promise((resolveP) => {
    const req = http.get(url, (res) => {
      res.resume();
      resolveP(res.statusCode ?? 0);
    });
    req.on("error", () => resolveP(0));
    req.setTimeout(2000, () => {
      req.destroy();
      resolveP(0);
    });
  });
}

async function waitHealthy() {
  // Pasion has a queue-worker/leader-election restart loop at cold boot
  // (queue_leader FK race). We require 5 consecutive passes plus a real
  // probe against its discovery endpoint to make sure the HTTP stack has
  // fully settled before returning.
  const targets = [
    ["pasion", "http://localhost:8090/.well-known/openid-configuration"],
    ["palpo", "http://localhost:8008/_matrix/client/versions"],
    ["padmin", "http://localhost:9090/"],
    ["element", "http://localhost:8080/"],
  ];
  const REQUIRED_CONSECUTIVE = 5;
  const deadline = Date.now() + 120_000;
  let streak = 0;
  while (Date.now() < deadline) {
    const statuses = await Promise.all(targets.map(([, u]) => probe(u)));
    const allUp = statuses.every((s) => s >= 200 && s < 500);
    if (allUp) {
      streak++;
      if (streak >= REQUIRED_CONSECUTIVE) {
        console.log(
          `[reset-stack] all services healthy (${streak} consecutive passes):`,
          Object.fromEntries(targets.map(([n], i) => [n, statuses[i]]))
        );
        return;
      }
    } else {
      streak = 0;
    }
    await new Promise((r) => setTimeout(r, 1000));
  }
  throw new Error("Services did not become healthy within 120s");
}

function psqlExec(sql) {
  const r = spawnSync(
    "docker",
    [
      "exec",
      "-i",
      "examples-postgres-1",
      "psql",
      "-U",
      "palpo",
      "-d",
      "pasion",
      "-tAc",
      sql,
    ],
    { encoding: "utf8", shell: false }
  );
  if (r.status !== 0) {
    throw new Error(`psql "${sql}" failed: ${r.stderr || r.stdout}`);
  }
  return r.stdout;
}

function dropPasionCrashLoopConstraint() {
  // Workarounds for pasion's queue worker crash loops (seen on the
  // `debug` target build shipped by examples/compose.yml):
  //
  //   1. queue_leader_queue_worker_id_fkey: at cold boot pasion's leader
  //      election tries to INSERT into queue_leader with a worker id that
  //      hasn't been persisted to queue_workers yet, tripping the FK and
  //      crashing the process. Dropping the FK breaks the crash loop.
  //
  //   2. mark_as_completed race: during load the queue occasionally fires
  //      "Expected 1 rows to be affected, but 0 rows were affected" on
  //      queue_jobs updates and shuts the server down. We can't patch
  //      pasion, but truncating queue state after the initial migrations
  //      gives the worker a clean slate and makes the race rarer.
  //
  // Must run while pasion is stopped — queue_leader has an exclusive lock
  // as soon as the worker loop is active, so ALTER TABLE will hang.
  console.log("[reset-stack] stopping pasion to release queue locks");
  run("docker", ["compose", ...COMPOSE_FILES, "stop", "pasion"]);

  console.log("[reset-stack] dropping queue_* FKs + truncating queue state");
  // See README "Known pasion-side issues" for why each of these matters.
  // In short: pasion's debug build races its own queue FKs on startup and
  // under load; dropping them avoids the crash loops without changing
  // observable behavior in the happy path.
  for (const sql of [
    "ALTER TABLE queue_leader DROP CONSTRAINT IF EXISTS queue_leader_queue_worker_id_fkey;",
    "ALTER TABLE queue_jobs DROP CONSTRAINT IF EXISTS queue_jobs_started_by_fkey;",
    "ALTER TABLE queue_jobs DROP CONSTRAINT IF EXISTS queue_jobs_next_attempt_id_fkey;",
    "ALTER TABLE queue_jobs DROP CONSTRAINT IF EXISTS queue_jobs_schedule_name_fkey;",
    "ALTER TABLE queue_schedules DROP CONSTRAINT IF EXISTS fk_schedule_last_job;",
  ]) {
    psqlExec(sql);
  }
  psqlExec("TRUNCATE queue_leader, queue_workers, queue_jobs CASCADE;");

  console.log("[reset-stack] starting pasion back up");
  run("docker", ["compose", ...COMPOSE_FILES, "start", "pasion"]);
}

(async () => {
  if (mode === "down") {
    console.log("[reset-stack] docker compose down -v");
    run("docker", ["compose", ...COMPOSE_FILES, "down", "-v"]);
    return;
  }
  if (mode === "reset") {
    console.log("[reset-stack] docker compose down -v");
    run("docker", ["compose", ...COMPOSE_FILES, "down", "-v"]);
  }
  console.log("[reset-stack] docker compose up -d");
  run("docker", ["compose", ...COMPOSE_FILES, "up", "-d"]);
  // Let postgres apply its migrations + pasion write its initial schema
  // before we start patching queue state.
  await new Promise((r) => setTimeout(r, 10_000));
  dropPasionCrashLoopConstraint();
  await waitHealthy();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});

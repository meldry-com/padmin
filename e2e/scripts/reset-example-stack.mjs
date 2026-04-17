#!/usr/bin/env node
// Stack reset helper. Commands:
//   reset-example-stack.mjs         -> down -v + up -d + wait healthy
//   reset-example-stack.mjs up      -> up -d + wait healthy
//   reset-example-stack.mjs down    -> down -v

import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import http from "node:http";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_CWD = resolve(__dirname, "..", "..");
const COMPOSE_FILES = [
  "-f",
  "examples/compose.yml",
  "-f",
  "e2e/fixtures/example-stack/compose.override.yml",
];

const mode = process.argv[2] ?? "reset";

function run(cmd, args, opts = {}) {
  const result = spawnSync(cmd, args, {
    cwd: REPO_CWD,
    stdio: "inherit",
    shell: false,
    ...opts,
  });

  if (result.status !== 0) {
    throw new Error(`${cmd} ${args.join(" ")} exited ${result.status}`);
  }
}

function probe(url) {
  return new Promise((resolveProbe) => {
    const request = http.get(url, (response) => {
      response.resume();
      resolveProbe(response.statusCode ?? 0);
    });

    request.on("error", () => resolveProbe(0));
    request.setTimeout(2_000, () => {
      request.destroy();
      resolveProbe(0);
    });
  });
}

async function waitHealthy() {
  const targets = [
    ["pasion", "http://localhost:7080/.well-known/openid-configuration"],
    ["palpo", "http://localhost:8008/_matrix/client/versions"],
    ["padmin", "http://localhost:7060/"],
    ["element", "http://localhost:7070/"],
  ];
  const requiredConsecutive = 5;
  const deadline = Date.now() + 120_000;
  let streak = 0;

  while (Date.now() < deadline) {
    const statuses = await Promise.all(targets.map(([, url]) => probe(url)));
    const allUp = statuses.every((status) => status >= 200 && status < 500);

    if (allUp) {
      streak++;
      if (streak >= requiredConsecutive) {
        console.log(
          `[reset-example-stack] all services healthy (${streak} consecutive passes):`,
          Object.fromEntries(targets.map(([name], index) => [name, statuses[index]]))
        );
        return;
      }
    } else {
      streak = 0;
    }

    await new Promise((resolveSleep) => setTimeout(resolveSleep, 1_000));
  }

  throw new Error("Services did not become healthy within 120s");
}

function psqlExec(sql) {
  const result = spawnSync(
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
    {
      cwd: REPO_CWD,
      encoding: "utf8",
      shell: false,
    }
  );

  if (result.status !== 0) {
    throw new Error(`psql "${sql}" failed: ${result.stderr || result.stdout}`);
  }

  return result.stdout;
}

function dropPasionCrashLoopConstraints() {
  console.log("[reset-example-stack] stopping pasion to release queue locks");
  run("docker", ["compose", ...COMPOSE_FILES, "stop", "pasion"]);

  console.log("[reset-example-stack] dropping queue_* FKs + truncating queue state");
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

  console.log("[reset-example-stack] starting pasion back up");
  run("docker", ["compose", ...COMPOSE_FILES, "start", "pasion"]);
}

(async () => {
  if (mode === "down") {
    console.log("[reset-example-stack] docker compose down -v");
    run("docker", ["compose", ...COMPOSE_FILES, "down", "-v"]);
    return;
  }

  if (mode === "reset") {
    console.log("[reset-example-stack] docker compose down -v");
    run("docker", ["compose", ...COMPOSE_FILES, "down", "-v"]);
  }

  console.log("[reset-example-stack] docker compose up -d");
  run("docker", ["compose", ...COMPOSE_FILES, "up", "-d"]);
  await new Promise((resolveSleep) => setTimeout(resolveSleep, 10_000));
  dropPasionCrashLoopConstraints();
  await waitHealthy();
})().catch((error) => {
  console.error(error);
  process.exit(1);
});

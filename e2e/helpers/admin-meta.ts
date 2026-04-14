import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

export type AdminMeta = {
  username: string;
  password: string;
  userId: string;
  email: string;
  displayName: string;
};

export const ADMIN_META_PATH = path.resolve(process.cwd(), "e2e/.auth/admin-meta.json");

export async function writeAdminMeta(meta: AdminMeta): Promise<void> {
  await mkdir(path.dirname(ADMIN_META_PATH), { recursive: true });
  await writeFile(ADMIN_META_PATH, JSON.stringify(meta, null, 2), "utf8");
}

export async function readAdminMeta(): Promise<AdminMeta> {
  const content = await readFile(ADMIN_META_PATH, "utf8");
  return JSON.parse(content) as AdminMeta;
}

import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";

export async function pickDirectory(title: string): Promise<string | undefined> {
  const selection = await open({ directory: true, multiple: false, title });

  return typeof selection === "string" ? selection : undefined;
}

export function assetUrl(path: string): string {
  return convertFileSrc(path);
}

export async function revealPath(path: string): Promise<void> {
  await openPath(path);
}

export async function browse(url: string): Promise<void> {
  await openUrl(url);
}

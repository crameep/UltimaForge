import { isTauri } from "@tauri-apps/api/core";
import { message } from "@tauri-apps/plugin-dialog";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { isRunningAsAdmin, relaunchAsAdmin } from "./api";

export type LauncherUpdateCheck = {
  updateAvailable: boolean;
  version?: string;
  notes?: string;
  date?: string;
  error?: string;
  /** Call to download, install, and relaunch. Only present when updateAvailable is true. */
  install?: () => Promise<void>;
};

type LauncherUpdateOptions = {
  interactive?: boolean;
  /** Silent check, but surface update info for the UI to display a modal. */
  promptIfAvailable?: boolean;
};

export async function checkForLauncherUpdate(
  options: LauncherUpdateOptions = {}
): Promise<LauncherUpdateCheck> {
  const { interactive = false } = options;

  if (!isTauri()) {
    return { updateAvailable: false };
  }

  try {
    const update = await check();

    if (!update) {
      if (interactive) {
        await message("You're up to date.", {
          title: "Launcher Updates",
          kind: "info",
        });
      }
      return { updateAvailable: false };
    }

    const install = async () => {
      await update.downloadAndInstall();
      // Preserve elevation through the relaunch — Tauri's plain relaunch()
      // spawns a non-elevated process, losing admin rights. If we're currently
      // elevated, use runas so the new instance starts elevated too (no extra
      // UAC prompt since the parent is already elevated).
      let elevated = false;
      try { elevated = await isRunningAsAdmin(); } catch { /* assume not */ }
      if (elevated) {
        await relaunchAsAdmin();
      } else {
        await relaunch();
      }
    };

    return {
      updateAvailable: true,
      version: update.version,
      notes: update.body ?? "",
      date: update.date ?? "",
      install,
    };
  } catch (error) {
    const messageText =
      typeof error === "string" ? error
      : error instanceof Error ? error.message
      : "Failed to check for updates";

    if (interactive) {
      await message(messageText, {
        title: "Launcher Updates",
        kind: "error",
      });
    }

    return { updateAvailable: false, error: messageText };
  }
}

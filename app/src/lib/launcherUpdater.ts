import { isTauri } from "@tauri-apps/api/core";
import { confirm, message } from "@tauri-apps/plugin-dialog";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type LauncherUpdateCheck = {
  updateAvailable: boolean;
  version?: string;
  notes?: string;
  date?: string;
  error?: string;
};

type LauncherUpdateOptions = {
  interactive?: boolean;
};

function buildPromptMessage(version?: string, notes?: string, date?: string) {
  const lines = [
    "A launcher update is available.",
    version ? `Version: ${version}` : "",
    date ? `Published: ${date}` : "",
    "",
    notes ? "Release notes:" : "",
    notes || "",
    "",
    "Install now and restart?",
  ].filter(Boolean);

  return lines.join("\n");
}

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

    const notes = update.body ?? "";
    const date = update.date ?? "";
    const prompt = buildPromptMessage(update.version, notes, date);

    if (interactive) {
      const shouldInstall = await confirm(prompt, {
        title: "Launcher Update Available",
        okLabel: "Update and Restart",
        cancelLabel: "Later",
      });

      if (shouldInstall) {
        await update.downloadAndInstall();
        await relaunch();
      }
    }

    return {
      updateAvailable: true,
      version: update.version,
      notes,
      date,
    };
  } catch (error) {
    const messageText =
      error instanceof Error ? error.message : "Failed to check for updates";

    if (interactive) {
      await message(messageText, {
        title: "Launcher Updates",
        kind: "error",
      });
    }

    return { updateAvailable: false, error: messageText };
  }
}

import { relaunch } from "@tauri-apps/plugin-process";
import { check, Update } from "@tauri-apps/plugin-updater";

export async function checkForUpdate() {
    if (!import.meta.env.DEV) {
        return await check();
    }
}

export async function startUpdate(update: Update, setProgress: (progress: string) => any) {
    let downloaded = 0;
    let contentLength = 0;

    await update.downloadAndInstall(event => {
        switch (event.event) {
            case "Started":
                contentLength = event.data.contentLength ?? 0;
                setProgress(`Updating... 0%`);
                break;
            case "Progress":
                downloaded += event.data.chunkLength;
                const pct = (downloaded / contentLength * 100).toFixed(0);
                setProgress(`Updating... ${pct}%`);
                break;
            case "Finished":
                setProgress(`Download complete. Installing...`);
                break;
        }
    });

    setProgress("Update done");
    await relaunch();
    setProgress("");
}

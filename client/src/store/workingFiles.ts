import { createStore, produce } from "solid-js/store";
import { DatDescriptor, DatLanguage, DatWithLang } from "../bindings";
import { commands, FileNotification } from "../bindings";
import { listen } from "@tauri-apps/api/event";
import { createEffect, createSignal } from "solid-js";
import { createFoldersStore } from "./folders";
import { unwrap as unwrapResult } from "../util";

type DatDescriptorNames = DatDescriptor["type"];
type WorkingFilesState = {
  [name in DatDescriptorNames]?: {
    [lang: number]: {
      [key: number]: boolean
    }
  }
}

function langToNum(lang: DatLanguage) {
  switch (lang) {
    case "English":
      return 0;
    case "Japanese":
      return 1;
  }
}

export function createWorkingFilesStore(
  folders: ReturnType<typeof createFoldersStore>
) {
  const [workingFiles, setWorkingFiles] = createStore<WorkingFilesState>({});
  const [projectPath, setProjectPath] = createSignal();

  const setWorkingFileFromDescriptor = (dat: DatWithLang, is_delete: boolean) => {
    if (is_delete) {
      console.log("Got delete for", dat);
    }

    const descriptor = dat.descriptor;
    setWorkingFiles(
      produce((files) => {
        const _type = files[descriptor.type] = files[descriptor.type] ?? {}
        const langNum = langToNum(dat.lang)
        const lastLevel = _type[langNum] = _type[langNum] ?? {};
        lastLevel["index" in descriptor ? descriptor.index : 0] = !is_delete;
      })
    );
  };

  createEffect(() => {
    if (folders.getProjectFolder() != projectPath()) {
      setProjectPath(folders.getProjectFolder());
      setWorkingFiles({});

      commands.getWorkingFiles().then((loadedWorkingFiles) => {
        setWorkingFiles({});
        for (const dat of unwrapResult(loadedWorkingFiles)) {
          setWorkingFileFromDescriptor(dat, false);
        }
      });
    }
  });

  // Listen for changes to the YAML data files
  listen<FileNotification>("file-change", (event) => {
    const payload = event.payload;
    setWorkingFileFromDescriptor(payload.dat, payload.is_delete);
  });

  return {
    hasWorkingFile: (descriptor: DatDescriptor, lang: DatLanguage): boolean => {
      const kind = workingFiles[descriptor.type];
      const key = "index" in descriptor ? descriptor.index : 0;
      return kind?.[langToNum(lang)]?.[key] ?? false;
    }
  };
}


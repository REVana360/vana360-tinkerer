import { useData } from "../store";
import Updater from "./Updater";

function Statusbar() {
  const {
    folders: {
      getDatFolder,
      setDatFolder,
      getProjectFolder,
      setProjectFolder,
      promptDatFolder,
      promptProjectFolder,
    },
  } = useData();

  return (
    <footer class="flex bg-slate-900 border-t border-t-slate-400 text-slate-100 justify-start p-1 text-sm">
      <div class="flex w-full h-full items-center">
        <div
          class="items-center grid"
          style={"grid-template-columns: min-content max-content; height: min-content"}
        >
          <div class="px-1 text-right">Project:</div>

          <div
            class="cursor-pointer"
            onclick={(e) => {
              if (e.ctrlKey) {
                setProjectFolder(null);
              } else {
                promptProjectFolder();
              }
            }}
          >
            {getProjectFolder() ? (
              <div class="text-green-200">{getProjectFolder()}</div>
            ) : (
              <div class="underline text-red-200">
                None. Click to select one.
              </div>
            )}
          </div>
          <div class="px-1 text-right">FFXI:</div>
          <div
            class="cursor-pointer"
            onclick={(e) => {
              if (e.ctrlKey) {
                setDatFolder(null);
              } else {
                promptDatFolder();
              }
            }}
          >
            {getDatFolder() ? (
              <div class="text-green-200">{getDatFolder()}</div>
            ) : (
              <div class="underline text-red-200">
                None. Click here to select.
              </div>
            )}
          </div>
        </div>
        <div class="flex-grow"></div>
        <Updater></Updater>
      </div>
    </footer>
  );
}

export default Statusbar;

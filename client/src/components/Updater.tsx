import { createResource, createSignal, Match, Switch } from "solid-js";
import { checkForUpdate, startUpdate } from "../updating";
import { getVersion } from "@tauri-apps/api/app";

const Updater = () => {
    const [update] = createResource(checkForUpdate);
    const [appVersion] = createResource(getVersion);

    const [getProgress, setProgress] = createSignal<string>("");

    return (
        <div class="px-2 flex items-center justify-end">
            <Switch>
                <Match when={getProgress()}>
                    Updating {getProgress()}
                </Match>
                <Match when={!getProgress() && !update.loading && update()}>

                    <div class="pr-2">An update is available for XI Tinkerer ({update()?.version})</div>
                    <div class="pr-4">
                        <button
                            class="button accept w-full"
                            onClick={async () => {
                                await startUpdate(update()!, setProgress);
                            }}
                        >
                            Update
                        </button>
                    </div>
                </Match>
            </Switch>
            <div class="text-sm italic font-mono text-slate-300">v{appVersion()}</div>
        </div>
    );
};

export default Updater;

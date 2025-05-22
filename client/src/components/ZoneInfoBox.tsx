import { createSignal, Show } from "solid-js";
import { BlockInfo, PreparedPlacementData } from "../graphics/ximesh";

export interface TargetInfo {
    x: number;
    y: number;
    z: number;
    mapId: number;
    material: number;
    cellIdx: number;
    cellEntryIdx: number;
    block: BlockInfo;
    placement: PreparedPlacementData;
}

export interface ZoneInfoBoxProps {
    targetInfo: TargetInfo | undefined;
}

export function ZoneInfoBox(props: ZoneInfoBoxProps) {
    const [getShowDetails, setShowDetails] = createSignal<boolean>(true);

    return (
        <div class="h-full absolute right-0 top-0 overflow-y-auto m-0 p-0 pointer-events-none noselect z-50" style={{ "width": "20%", "min-width": "12rem" }}>
            <div class="w-full bg-black bg-opacity-90 m-0 rounded-sm pointer-events-auto font-mono">
                <div onClick={() => setShowDetails(!getShowDetails())} class="cursor-pointer px-2 py-1 font-bold">
                    <span class="font-mono">{getShowDetails() ? "—" : "▼"}</span> Info box
                </div>
                <hr></hr>

                <Show when={getShowDetails()}>
                    <div class="px-2">
                        <Show when={props.targetInfo} fallback={"No data"}>
                            <div>X: {props.targetInfo!.x.toFixed(3)}</div>
                            <div>Y: {props.targetInfo!.y.toFixed(3)}</div>
                            <div>Z: {props.targetInfo!.z.toFixed(3)}</div>
                            <div>Material: {props.targetInfo!.material}</div>
                            <div>Map ID: {props.targetInfo!.mapId}</div>
                            <div>CellIdx: {props.targetInfo!.cellIdx}</div>
                            <div>EntryIdx: {props.targetInfo!.cellEntryIdx}</div>
                            <div>Barrier block: {props.targetInfo!.block.hasBarriers ? "true" : "false"}</div>
                            <div>Placement Y: {props.targetInfo!.placement.yMin.toFixed(3)} to {props.targetInfo!.placement.yMax.toFixed(3)}</div>
                        </Show>
                    </div>
                </Show>
            </div>
        </div>);
}
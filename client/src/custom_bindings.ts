import { DatDescriptor, Result } from './bindings'
import { invoke as TAURI_INVOKE } from "@tauri-apps/api/core";
declare global {
    interface Window {
        __TAURI_INVOKE__<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
    }
}

export async function getZoneModel(datDescriptor: DatDescriptor) : Promise<Result<ZoneData, any>> {
    try {
        return { status: "ok", data: await TAURI_INVOKE("get_zone_model", { datDescriptor }) };
    } catch (e) {
        if(e instanceof Error) throw e;
        else return { status: "error", error: e  as any };
    }
}



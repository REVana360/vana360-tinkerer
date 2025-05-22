import { Result } from './bindings'
import { invoke as TAURI_INVOKE } from "@tauri-apps/api/core";
declare global {
    interface Window {
        __TAURI_INVOKE__<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
    }
}

export async function getZoneModel(zoneId: number) : Promise<Result<any, any>> {
    try {
        return { status: "ok", data: await TAURI_INVOKE("get_zone_model", { zoneId }) };
    } catch (e) {
        if(e instanceof Error) throw e;
        else return { status: "error", error: e  as any };
    }
}



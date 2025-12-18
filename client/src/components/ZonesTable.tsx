import Table from "./Table";
import { unwrap } from "../util";
import { commands } from "../bindings";

function ZonesTable() {

  return (
    <Table
      title="Zones"
      rowsResourceFetcher={async () => unwrap(await commands.getZonesForType({ type: "ZoneData", index: 0 }))}
      columns={[{ name: "Name", key: "name" }, { name: "ID", key: "id" }]}
      defaultSortColumn="name"
      additionalColumns={[
        {
          name: "DAT path",
          content: (row) => (<span class="font-mono text-sm">{row.dat_path}</span>)
        },
        {
          name: "Collision .obj",
          content: (row) => (
            <span
              class="clickable pl-2 font-mono"
              onclick={async () => unwrap(await commands.zoneToWavefront(row.id, "Collision"))}
            >
              Export
            </span>)
        },
        {
          name: "Model .obj",
          content: (row) => (
            <span
              class="clickable pl-2 font-mono"
              onclick={async () => unwrap(await commands.zoneToWavefront(row.id, "Model"))}
            >
              Export
            </span>)
        },
        {
          name: "View",
          content: (row) => (<a href={"/zones/" + row.id} class="text-blue-400 font-semibold">View</a>)
        },
      ]}
      headerActions={
        [<div class="flex gap-3 items-baseline">
          <span>Export all wavefront (.obj):</span>
          <button onClick={async () => unwrap(await commands.allZonesToWavefront("Collision"))}>Collision</button>
          <button onClick={async () => unwrap(await commands.allZonesToWavefront("Model"))}>Model</button>
        </div>]
      }
    />
  );
}

export default ZonesTable;

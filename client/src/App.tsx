import Sidebar, { NavItem } from "./components/Sidebar";
import Statusbar from "./components/Statusbar";
import Home from "./components/Home";
import { Routes, Route } from "@solidjs/router";
import { HiSolidAdjustmentsHorizontal, HiSolidChatBubbleLeftRight, HiSolidCog8Tooth, HiSolidMagnifyingGlass, HiSolidMap, HiSolidPencilSquare, HiSolidShoppingBag, HiSolidUser } from "solid-icons/hi";
import DatTable from "./components/DatTable";
import Table from "./components/Table";
import ZoneData from "./components/ZoneData";
import { commands } from "./bindings";
import Logs from "./components/Logs";
import { unwrap } from "./util";

const navItems: NavItem[] = [
  {
    name: "Home",
    path: "/",
    icon: <HiSolidCog8Tooth />,
  },
  {
    name: "Browse",
    path: "/browse",
    icon: <HiSolidMagnifyingGlass />,
  },
  { header: "Strings" },
  {
    name: "String tables",
    path: "/strings",
    icon: <HiSolidPencilSquare />,
  },
  {
    name: "Global dialog",
    path: "/global_dialog",
    icon: <HiSolidChatBubbleLeftRight />,
  },

  { header: "By zone" },
  {
    name: "Zone Data",
    path: "/zones",
    icon: <HiSolidMap />,
  },
  {
    name: "Entity names",
    path: "/entities",
    icon: <HiSolidUser />,
  },
  {
    name: "Dialog",
    path: "/dialog",
    icon: <HiSolidChatBubbleLeftRight />,
  },
  {
    name: "Dialog (2)",
    path: "/dialog2",
    icon: <HiSolidChatBubbleLeftRight />,
  },

  { header: "Other" },
  {
    name: "Items",
    path: "/items",
    icon: <HiSolidShoppingBag />,
  },
  {
    name: "Misc.",
    path: "/misc",
    icon: <HiSolidAdjustmentsHorizontal />,
  },

];

function App() {
  return (
    <main class="h-screen w-screen">
      <div class="flex flex-row w-screen">
        <Sidebar navItems={navItems} />

        <div class="flex flex-col flex-grow min-w-0 basis-0 h-screen">
          <div class="content flex-grow overflow-y-auto">
            <Routes>
              <Route path="/" component={Home}></Route>

              <Route
                path="/browse"
                component={() => (
                  <Table
                    title="Browse"
                    rowsResourceFetcher={async () => unwrap(await commands.browseDats())}
                    columns={[{ name: "Name", key: "path" }, { name: "ID", key: "id" }]}
                    defaultSortColumn="path"
                  />
                )}
              ></Route>

              <Route
                path="/strings"
                component={() => (
                  <DatTable
                    title="Strings"
                    rowsResourceFetcher={async () => unwrap(await commands.getStandaloneStringDats())}
                    columns={[{ name: "Name", key: "type" }]}
                    defaultSortColumn="type"
                    toDatDescriptor={(datDescriptor) => datDescriptor}
                  />
                )}
              ></Route>

              <Route
                path="/items"
                component={() => (
                  <DatTable
                    title="Items"
                    rowsResourceFetcher={async () => unwrap(await commands.getItemDats())}
                    columns={[{ name: "Name", key: "type" }]}
                    defaultSortColumn="type"
                    toDatDescriptor={(datDescriptor) => datDescriptor}
                  />
                )}
              ></Route>

              <Route
                path="/misc"
                component={() => (
                  <DatTable
                    title="Misc."
                    rowsResourceFetcher={async () => unwrap(await commands.getMiscDats())}
                    columns={[{ name: "Name", key: "type" }]}
                    defaultSortColumn="type"
                    toDatDescriptor={(datDescriptor) => datDescriptor}
                  />
                )}
              ></Route>

              <Route
                path="/global_dialog"
                component={() => (
                  <DatTable
                    title="Global Dialog"
                    rowsResourceFetcher={async () => unwrap(await commands.getGlobalDialogDats())}
                    columns={[{ name: "Name", key: "type" }]}
                    defaultSortColumn="type"
                    toDatDescriptor={(datDescriptor) => datDescriptor}
                  />
                )}
              ></Route>

              <Route path="/zones">
                <Route
                  path="/"
                  component={() => (
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
                          name: "View",
                          content: (row) => (<a href={"/zones/" + row.id} class="text-blue-400 font-semibold">View</a>)
                        }
                      ]}
                    />
                  )}>
                </Route>

                <Route path="/:id" component={ZoneData}></Route>
              </Route>

              <Route
                path="/entities"
                component={() => (
                  <DatTable
                    title="Entity Names"
                    rowsResourceFetcher={async () => unwrap(await commands.getZonesForType({ type: "EntityNames", index: 0 }))}
                    columns={[{ name: "Name", key: "name" }, { name: "ID", key: "id" }]}
                    defaultSortColumn="name"
                    toDatDescriptor={(zone) => ({ type: "EntityNames", index: zone.id })}
                  />
                )}
              ></Route>

              <Route
                path="/dialog"
                component={() => (
                  <DatTable
                    title="Dialog"
                    rowsResourceFetcher={async () => unwrap(await commands.getZonesForType({ type: "Dialog", index: 0 }))}
                    columns={[{ name: "Name", key: "name" }, { name: "ID", key: "id" }]}
                    defaultSortColumn="name"
                    toDatDescriptor={(zone) => ({ type: "Dialog", index: zone.id })}
                  />
                )}
              ></Route>

              <Route
                path="/dialog2"
                component={() => (
                  <DatTable
                    title="Dialog (2)"
                    rowsResourceFetcher={async () => unwrap(await commands.getZonesForType({ type: "Dialog2", index: 0 }))}
                    columns={[{ name: "Name", key: "name" }, { name: "ID", key: "id" }]}
                    defaultSortColumn="name"
                    toDatDescriptor={(zone) => ({ type: "Dialog2", index: zone.id })}
                  />
                )}
              ></Route>

              <Route
                path="/logs"
                component={Logs}
              ></Route>
            </Routes>
          </div>
          <Statusbar />
        </div>
      </div>
    </main>
  );
}

export default App;

import {
  For,
  Match,
  Show,
  Switch,
  batch,
  createMemo,
  createResource,
  createSignal,
  onMount,
} from "solid-js";
import { commands, DatDescriptor } from "../bindings";
import { useData } from "../store";
import { unwrap } from "../util";

interface DatTableProps<T> {
  title: string;
  rowsResourceFetcher: () => Promise<T[]>,
  columns: { name: string, getter: (t: T) => any }[],
  defaultSortColumn?: number,
  toDatDescriptor: (t: T) => DatDescriptor,
  hasJp?: (t: T) => boolean,
}

function DatTable<T>
  ({ title, rowsResourceFetcher, columns, defaultSortColumn, toDatDescriptor, hasJp: hasJpGiven }: DatTableProps<T>) {
  const {
    processing: { canProcess, isProcessing, processing },
    workingFiles: { hasWorkingFile }
  } = useData();

  const hasJp = hasJpGiven ?? (() => false);

  const [rowsResource] = createResource(rowsResourceFetcher, { initialValue: [] });

  const [sortBy, setSortBy] = createSignal<number>(defaultSortColumn ?? 0);
  const [sortAsc, setSortAsc] = createSignal<boolean>(true);
  const [filterBy, setFilterBy] = createSignal<string>("");

  const updateSort = (columnIdx: number) => {
    if (columnIdx == sortBy()) {
      setSortAsc(!sortAsc());
    } else {
      batch(() => {
        setSortBy(columnIdx);
        setSortAsc(true);
      })
    }
  };

  const filteredRows = createMemo(() => {
    if (filterBy()) {
      const filterVal = filterBy();
      return rowsResource()
        .filter((e) => {
          return columns.find(c => {
            return ("" + c.getter(e)).toLowerCase().includes(filterVal.toLowerCase());
          })
        });
    } else {
      return [...(rowsResource())];
    }
  })

  const rows = createMemo(() => {
    const sortByGetter = columns[sortBy()].getter;
    const sortAscVal = sortAsc();
    const dir = sortAscVal ? 1 : -1;
    return filteredRows().sort((a, b) => {
      const aValue = sortByGetter(a);
      const bValue = sortByGetter(b);
      if (aValue < bValue) {
        return -1 * dir;
      } else if (aValue > bValue) {
        return 1 * dir;
      }
      return 0;
    }).slice(0, 1000);
  });


  // Make YAML
  const makeAllYaml = () => {
    if (!canProcess()) {
      return;
    }

    rows().forEach((row) => {
      commands.makeYaml(toDatDescriptor(row), "English");
      commands.makeYaml(toDatDescriptor(row), "Japanese");
    });
  };

  const makingYamlCount = createMemo(() => {
    return rows()
      .filter((row) => {
        const descriptor = toDatDescriptor(row);
        const key = "index" in descriptor ? descriptor.index : 0;
        return processing.Yaml?.[descriptor.type]?.[key] == true;
      })
      .length;
  });

  // Making DAT files from YAML
  const makeAllDats = () => {
    if (!canProcess()) {
      return;
    }

    rows().forEach((row) => {
      commands.makeDat(toDatDescriptor(row), "English");
    });
  };

  const makingDatCount = createMemo(() => {
    return rows()
      .filter((row) => {
        const descriptor = toDatDescriptor(row);
        const key = "index" in descriptor ? descriptor.index : 0;
        return processing.Dat?.[descriptor.type]?.[key] == true;
      })
      .length;
  });

  let inputRef: HTMLInputElement;
  onMount(() => {
    inputRef.focus();
  });

  return (
    <div class="w-full">
      <h1>{title}</h1>
      <hr />

      <div>
        <div class="flex flex-row space-x-5">
          <input
            class="mt-3"
            placeholder="Filter"
            ref={inputRef!}
            oninput={(e) => setFilterBy(e.target.value ?? "")}
          />

          <button
            disabled={makingYamlCount() > 0 || !canProcess()}
            onclick={() => makeAllYaml()}
          >
            Export all DATs
          </button>

          <button
            disabled={makingDatCount() > 0 || !canProcess()}
            onclick={() => makeAllDats()}
          >
            Make all DATs
          </button>
        </div>

        <Show when={!rowsResource.loading} fallback={<div>Loading...</div>}>
          <table class="table-auto">
            <thead>
              <tr>
                <For each={columns}>
                  {(col, idx) => (<th
                    class="hover:cursor-pointer"
                    onclick={() => updateSort(idx())}
                  >
                    {col.name}
                  </th>)}
                </For>
                <th class="w-40">Export from DAT</th>
                <th class="w-40">Generate DAT</th>
              </tr>
            </thead>

            <tbody>
              <For each={rows()}>
                {(row) => {
                  const descriptor = toDatDescriptor(row);
                  return (
                    <tr class="hover:bg-slate-700">
                      <For each={columns}>
                        {(col) => <td>{col.getter(row)}</td>}
                      </For>

                      <Show
                        when={canProcess()}
                        fallback={
                          <td colSpan={2}>
                            <span class="italic text-sm">
                              Select a project and FFXI folder.
                            </span>
                          </td>
                        }
                      >
                        <td>
                          <Switch>
                            <Match when={isProcessing("Yaml", descriptor)}>
                              <span class="italic">Exporting...</span>
                            </Match>

                            <Match when={true}>
                              <span
                                class="clickable pl-2 font-mono"
                                onclick={async () => unwrap(await commands.makeYaml(descriptor, "English"))}
                              >
                                [EN]
                              </span>
                              <Show when={hasJp(row)}>
                                <span
                                  class="clickable pl-2 font-mono"
                                  onclick={async () => unwrap(await commands.makeYaml(descriptor, "Japanese"))}
                                >
                                  [JP]
                                </span>
                              </Show>
                            </Match>
                          </Switch>
                        </td>
                        <td>
                          <Switch>
                            <Match when={isProcessing("Dat", descriptor)}>
                              <span class="italic">Making...</span>
                            </Match>

                            <Match when={true}>
                              <Show when={hasWorkingFile(descriptor, "English")}>
                                <span
                                  class="clickable pl-2 font-mono"
                                  onclick={async () => unwrap(await commands.makeDat(descriptor, "English"))}
                                >
                                  [EN]
                                </span>
                              </Show>
                              <Show when={hasJp(row) && hasWorkingFile(descriptor, "Japanese")}>
                                <span
                                  class="clickable pl-2 font-mono"
                                  onclick={async () => unwrap(await commands.makeDat(descriptor, "Japanese"))}
                                >
                                  [JP]
                                </span>
                              </Show>
                            </Match>
                          </Switch>
                        </td>
                      </Show>
                    </tr>
                  )
                }}
              </For>
            </tbody>
          </table>
        </Show>
      </div>
    </div>
  );
}

export default DatTable;

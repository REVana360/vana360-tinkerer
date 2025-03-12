import {
  For,
  JSX,
  Show,
  batch,
  createMemo,
  createResource,
  createSignal,
  onMount,
} from "solid-js";

interface AdditionalColumn<T> {
  name: string;
  content: (row: T) => JSX.Element;
}

interface TableProps<
  T extends { [key in Column]: any },
  Column extends keyof T
> {
  title: string;
  rowsResourceFetcher: () => Promise<T[]>;
  headerActions?: JSX.Element[];
  additionalColumns?: AdditionalColumn<T>[];
  columns: { name: string; key: Column }[];
  defaultSortColumn: Column;
}

function Table<
  T extends { [key in Column]: any },
  Column extends keyof T & string
>({
  title,
  rowsResourceFetcher,
  columns,
  defaultSortColumn,
  headerActions,
  additionalColumns,
}: TableProps<T, Column>) {
  const [rowsResource] = createResource(rowsResourceFetcher, {
    initialValue: [],
  });

  const [sortBy, setSortBy] = createSignal<Column>(defaultSortColumn);
  const [sortAsc, setSortAsc] = createSignal<boolean>(true);
  const [filterBy, setFilterBy] = createSignal<string>("");

  const updateSort = (column: Column) => {
    if (column == sortBy()) {
      setSortAsc(!sortAsc());
    } else {
      batch(() =>{
        setSortBy(column as any);
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
            return ("" + e[c.key]).includes(filterVal);
        })
        });
    } else {
      return [...(rowsResource())];
    }
  })

  const rows = createMemo(() => {
    const sortByVal = sortBy();
    const sortAscVal = sortAsc();
    return filteredRows().sort((a, b) => {
      const aValue = a[sortByVal];
      const bValue = b[sortByVal];
      const dir = sortAscVal ? 1 : -1;
      if (aValue < bValue) {
        return -1 * dir;
      } else if (aValue > bValue) {
        return 1 * dir;
      }
      return 0;
    }).slice(0, 1000);
  });

  let inputRef: HTMLInputElement;
  onMount(() => {
    inputRef.focus();
  });

  return (
    <div class="w-full h-screen">
      <h1>{title}</h1>
      <hr />

      <div class="w-full">
        <div class="flex flex-row space-x-5">
          <input
            class="mt-3"
            placeholder="Filter"
            ref={inputRef!}
            oninput={(e) => setFilterBy(e.target.value ?? "")}
          />

          {headerActions}
        </div>

        <Show when={!rowsResource.loading} fallback={<div>Loading...</div>}>
          <table class="w-full h-full">
            <thead>
              <tr>
                {columns.map((col) => (
                  <th
                    class="hover:cursor-pointer"
                    onclick={() => updateSort(col.key)}
                  >
                    {col.name}
                  </th>
                ))}

                {additionalColumns
                  ? additionalColumns.map((col) => <th>{col.name}</th>)
                  : undefined}
              </tr>
            </thead>

            <tbody>
              <For each={rows()}>
                {(row) => {
                  return (
                    <tr class="hover:bg-slate-700">
                      {columns.map((col) => (
                        <td>{row[col.key]}</td>
                      ))}

                      {additionalColumns
                        ? additionalColumns.map((col) => (
                            <td>{col.content(row)}</td>
                          ))
                        : undefined}
                    </tr>
                  );
                }}
              </For>
            </tbody>
          </table>
        </Show>
      </div>
    </div>
  );
}

export default Table;

export function buildMatrix(doc, ev, paint) {
  const table = doc.createElement("table");
  table.className = "modui-matrix";
  table.style.borderCollapse = "collapse";
  table.style.whiteSpace = "nowrap";
  const addRow = (cells, tag) => {
    const tr = doc.createElement("tr");
    for (const text of cells) {
      const td = doc.createElement(tag);
      td.style.padding = "0 6px";
      td.style.textAlign = tag === "th" ? "center" : "left";
      paint(td, String(text));
      tr.appendChild(td);
    }
    table.appendChild(tr);
  };
  if (Array.isArray(ev.columns)) addRow(ev.columns, "th");
  for (const row of ev.rows) addRow(row, "td");
  return table;
}

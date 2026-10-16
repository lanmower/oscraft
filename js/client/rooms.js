// Room panel for the browser page. Plain ES module with no imports: play.html loads it directly.
// Lists the rooms announced to the page (window event "oscraft:rooms", detail.rooms = array of names)
// and joins the one picked or typed by reloading the page with that room in the hash.
const ROOM_NAME = /^[A-Za-z0-9_-]{1,64}$/;
const panel = document.getElementById("rooms");
const roomNames = new Set();
document.getElementById("room-current").textContent = location.hash.slice(1) || "lobby";

function joinNamedRoom(name) {
  if (!ROOM_NAME.test(name)) return;
  location.hash = name;
  location.reload();
}

function renderRoomList() {
  const items = [...roomNames].map((name) => {
    const button = document.createElement("button");
    button.type = "button";
    button.dataset.room = name;
    button.textContent = name;
    button.addEventListener("click", () => joinNamedRoom(name));
    const item = document.createElement("li");
    item.append(button);
    return item;
  });
  document.getElementById("room-list").replaceChildren(...items);
}

window.addEventListener("oscraft:rooms", (e) => {
  const rooms = Array.isArray(e.detail && e.detail.rooms) ? e.detail.rooms : [];
  for (const name of rooms) if (ROOM_NAME.test(name)) roomNames.add(name);
  renderRoomList();
});
panel.addEventListener("submit", (e) => {
  e.preventDefault();
  joinNamedRoom(document.getElementById("room-name").value.trim());
});
panel.dataset.ready = "1";

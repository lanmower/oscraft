export const MIDDLE_BUTTON = 1;
export const RIGHT_BUTTON = 2;

const DRAG_ACTION_BY_BUTTON = new Map([
  [MIDDLE_BUTTON, "rotate"],
  [RIGHT_BUTTON, "pan"],
]);

export function cameraDragAction(button, moved) {
  if (!moved) return null;
  return DRAG_ACTION_BY_BUTTON.get(button) ?? null;
}

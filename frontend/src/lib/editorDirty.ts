export type CloseEditorResult = { ok: true } | { ok: false; reason: "dirty" };

export function cloneSnapshot<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

export function isEditorFormDirty<T>(snapshot: T | null, current: T | null): boolean {
  if (snapshot === null && current === null) return false;
  if (snapshot === null || current === null) return true;
  return JSON.stringify(snapshot) !== JSON.stringify(current);
}

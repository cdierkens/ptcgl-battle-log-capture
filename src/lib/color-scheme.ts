interface SchemeQuery {
  addEventListener(type: "change", listener: () => void): void;
  readonly matches: boolean;
  removeEventListener(type: "change", listener: () => void): void;
}

/** Keeps `.dark` on `root` in step with the OS colour scheme (ADR 0003). Returns an unsubscribe. */
export function followSystemColorScheme(
  root: { classList: { toggle(token: string, force: boolean): unknown } },
  query: SchemeQuery,
): () => void {
  const apply = () => root.classList.toggle("dark", query.matches);
  apply();
  query.addEventListener("change", apply);
  return () => query.removeEventListener("change", apply);
}

export function resolveLayoutProbeSurface(
  search: string,
  allowLayoutProbe: boolean,
) {
  if (!allowLayoutProbe) {
    return null;
  }

  return new URLSearchParams(search).get("layout-probe");
}

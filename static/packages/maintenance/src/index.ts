// The maintenance page of `hermetic serve`: <hermetic-unused-assets>.

import { HermeticUnusedAssets } from "./element";

export { HermeticUnusedAssets };

export function define(name = "hermetic-unused-assets"): void {
  if (!customElements.get(name)) customElements.define(name, HermeticUnusedAssets);
}

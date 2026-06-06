import { create } from "zustand";
import {
  stackCatalog,
  stackPremade,
  stackList,
  stackSet,
  stackApplyPremade,
  type CatalogOption,
  type PremadeStack,
  type Selection,
} from "../api/stack";

interface StackStore {
  catalog: Record<string, CatalogOption[]>;
  premade: PremadeStack[];
  selections: Record<number, Selection[] | undefined>;
  error: Record<number, string | null>;
  load: (id: number) => Promise<void>;
  set: (id: number, pane: string, choice: string) => Promise<void>;
  applyPremade: (id: number, name: string) => Promise<void>;
}

export const useStackStore = create<StackStore>((setState, get) => ({
  catalog: {},
  premade: [],
  selections: {},
  error: {},

  load: async (id) => {
    try {
      // Catalog + pre-made stacks are global; fetch once.
      if (Object.keys(get().catalog).length === 0) {
        const [catalog, premade] = await Promise.all([stackCatalog(), stackPremade()]);
        setState({ catalog, premade });
      }
      const selections = await stackList(id);
      setState((s) => ({ selections: { ...s.selections, [id]: selections }, error: { ...s.error, [id]: null } }));
    } catch (e) {
      setState((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
  },

  set: async (id, pane, choice) => {
    try {
      await stackSet(id, pane, choice);
    } catch (e) {
      setState((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
    const selections = await stackList(id).catch(() => undefined);
    if (selections) setState((s) => ({ selections: { ...s.selections, [id]: selections } }));
  },

  applyPremade: async (id, name) => {
    try {
      await stackApplyPremade(id, name);
    } catch (e) {
      setState((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
    const selections = await stackList(id).catch(() => undefined);
    if (selections) setState((s) => ({ selections: { ...s.selections, [id]: selections } }));
  },
}));

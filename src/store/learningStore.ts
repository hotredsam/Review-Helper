import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";
import {
  type Subject,
  subjectsList,
  subjectCreate,
  subjectDelete,
} from "../api/learning";

type Status = "idle" | "loading" | "ready" | "error";

interface LearningState {
  subjects: Subject[];
  selectedSubjectId: number | null;
  status: Status;
  error: string | null;
  load: () => Promise<void>;
  create: (title: string, kind: "describe" | "upload", sourceText: string) => Promise<number>;
  select: (id: number | null) => void;
  remove: (id: number) => Promise<void>;
}

/**
 * Learning-mode state. Subjects are the database's source of truth (reloaded on
 * entry); only the selected subject is persisted so the same one reopens after a
 * restart. Mutations go through the Rust commands.
 */
export const useLearningStore = create<LearningState>()(
  persist(
    (set, get) => ({
      subjects: [],
      selectedSubjectId: null,
      status: "idle",
      error: null,

      load: async () => {
        set({ status: "loading", error: null });
        try {
          const subjects = await subjectsList();
          const sel = get().selectedSubjectId;
          const stillExists = sel != null && subjects.some((s) => s.id === sel);
          set({ subjects, status: "ready", selectedSubjectId: stillExists ? sel : null });
        } catch (e) {
          set({ status: "error", error: String(e) });
        }
      },

      create: async (title, kind, sourceText) => {
        const id = await subjectCreate(title, kind, sourceText);
        // Reload so the new subject (with its server-assigned fields) appears.
        await get().load();
        set({ selectedSubjectId: id });
        return id;
      },

      select: (id) => set({ selectedSubjectId: id }),

      remove: async (id) => {
        await subjectDelete(id);
        set((s) => ({
          subjects: s.subjects.filter((x) => x.id !== id),
          selectedSubjectId: s.selectedSubjectId === id ? null : s.selectedSubjectId,
        }));
      },
    }),
    {
      name: "review-helper.learning",
      storage: createJSONStorage(() => localStorage),
      partialize: (s) => ({ selectedSubjectId: s.selectedSubjectId }),
    },
  ),
);

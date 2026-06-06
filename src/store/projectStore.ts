import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";
import {
  type Project,
  listProjects,
  createProject,
  renameProject,
  deleteProject,
  importRepo as importRepoApi,
  linkRepoByUrl,
  createRepoProject,
  cloneProject,
} from "../api/projects";
import type { RepoSummary } from "../api/github";

type Status = "idle" | "loading" | "ready" | "error";

interface ProjectState {
  projects: Project[];
  activeProjectId: number | null;
  status: Status;
  error: string | null;
  cloneState: Record<number, "idle" | "cloning" | "done" | "error">;
  cloneError: Record<number, string | null>;
  load: () => Promise<void>;
  create: (name: string, kind: Project["kind"], appType?: string) => Promise<Project>;
  importRepo: (repo: RepoSummary) => Promise<Project>;
  linkUrl: (url: string) => Promise<Project>;
  createRepo: (name: string, isPrivate: boolean) => Promise<Project>;
  select: (id: number) => void;
  rename: (id: number, name: string) => Promise<void>;
  remove: (id: number) => Promise<void>;
  syncClone: (id: number) => Promise<void>;
}

/**
 * Project state. The list itself is the database's source of truth (reloaded on
 * launch); only the active selection is persisted to localStorage so the same
 * project is focused after a restart. Mutations go through the Rust commands.
 */
export const useProjectStore = create<ProjectState>()(
  persist(
    (set, get) => ({
      projects: [],
      activeProjectId: null,
      status: "idle",
      error: null,
      cloneState: {},
      cloneError: {},

      load: async () => {
        set({ status: "loading", error: null });
        try {
          const projects = await listProjects();
          const active = get().activeProjectId;
          const stillExists = active != null && projects.some((p) => p.id === active);
          set({
            projects,
            status: "ready",
            activeProjectId: stillExists ? active : projects[0]?.id ?? null,
          });
        } catch (e) {
          set({ status: "error", error: String(e) });
        }
      },

      create: async (name, kind, appType) => {
        const project = await createProject(name, kind, appType);
        set((s) => ({
          projects: [...s.projects, project],
          activeProjectId: project.id,
        }));
        return project;
      },

      importRepo: async (repo) => {
        const project = await importRepoApi(repo.full_name, repo.clone_url, repo.default_branch);
        set((s) => ({ projects: [...s.projects, project], activeProjectId: project.id }));
        return project;
      },

      linkUrl: async (url) => {
        const project = await linkRepoByUrl(url);
        set((s) => ({ projects: [...s.projects, project], activeProjectId: project.id }));
        return project;
      },

      createRepo: async (name, isPrivate) => {
        const project = await createRepoProject(name, isPrivate);
        set((s) => ({ projects: [...s.projects, project], activeProjectId: project.id }));
        return project;
      },

      select: (id) => set({ activeProjectId: id }),

      rename: async (id, name) => {
        const updated = await renameProject(id, name);
        set((s) => ({
          projects: s.projects.map((p) => (p.id === id ? updated : p)),
        }));
      },

      remove: async (id) => {
        await deleteProject(id);
        set((s) => {
          const projects = s.projects.filter((p) => p.id !== id);
          const activeProjectId =
            s.activeProjectId === id ? projects[0]?.id ?? null : s.activeProjectId;
          return { projects, activeProjectId };
        });
      },

      syncClone: async (id) => {
        set((s) => ({
          cloneState: { ...s.cloneState, [id]: "cloning" },
          cloneError: { ...s.cloneError, [id]: null },
        }));
        try {
          const updated = await cloneProject(id);
          set((s) => ({
            projects: s.projects.map((p) => (p.id === id ? updated : p)),
            cloneState: { ...s.cloneState, [id]: "done" },
          }));
        } catch (e) {
          set((s) => ({
            cloneState: { ...s.cloneState, [id]: "error" },
            cloneError: { ...s.cloneError, [id]: String(e) },
          }));
        }
      },
    }),
    {
      name: "review-helper.projects",
      storage: createJSONStorage(() => localStorage),
      partialize: (s) => ({ activeProjectId: s.activeProjectId }),
    },
  ),
);

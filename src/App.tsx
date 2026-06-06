import { useEffect, useState } from "react";
import { Sidebar } from "./components/Sidebar";
import { MainPane } from "./components/MainPane";
import { NewProjectDialog } from "./components/NewProjectDialog";
import { ModelBanner } from "./components/ModelBanner";
import { useProjectStore } from "./store/projectStore";
import { useStatusStore } from "./store/statusStore";

/**
 * App shell: loads projects from the database on mount, then renders the
 * hamburger nav + active pane. All colors come from theme tokens.
 */
function App() {
  const status = useProjectStore((s) => s.status);
  const error = useProjectStore((s) => s.error);
  const load = useProjectStore((s) => s.load);
  const projects = useProjectStore((s) => s.projects);
  const [dialogOpen, setDialogOpen] = useState(false);
  const refreshStatus = useStatusStore((s) => s.refresh);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    void refreshStatus();
  }, [refreshStatus]);

  if (status === "idle" || status === "loading") {
    return <CenterMessage text="Loading…" />;
  }
  if (status === "error") {
    return <CenterMessage text={`Couldn't open the database: ${error}`} tone="danger" />;
  }

  const openNewProject = () => setDialogOpen(true);

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-bg text-fg">
      <ModelBanner />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar onNewProject={openNewProject} hasProject={projects.length > 0} />
        <main className="flex-1 overflow-hidden">
          <MainPane onNewProject={openNewProject} />
        </main>
      </div>
      <NewProjectDialog open={dialogOpen} onClose={() => setDialogOpen(false)} />
    </div>
  );
}

function CenterMessage({ text, tone }: { text: string; tone?: "danger" }) {
  return (
    <div className="flex h-screen items-center justify-center bg-bg p-8 text-center">
      <p className={"text-sm " + (tone === "danger" ? "text-danger" : "text-fg-muted")}>{text}</p>
    </div>
  );
}

export default App;

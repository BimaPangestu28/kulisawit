import { useEffect, useState } from "react";
import { useProjects } from "@/hooks/useProjects";
import { ProjectSwitcher } from "@/components/ProjectSwitcher";
import { KanbanBoard } from "@/components/KanbanBoard";
import { EmptyState } from "@/components/EmptyState";

const STORAGE_KEY = "kulisawit.activeProject";

function App() {
  const [activeProjectId, setActiveProjectId] = useState<string | null>(() =>
    localStorage.getItem(STORAGE_KEY),
  );
  const { data: projects } = useProjects();

  // Auto-select first project on mount if nothing persisted and projects exist
  useEffect(() => {
    if (activeProjectId === null && projects && projects.length > 0) {
      const firstId = projects[0].id;
      setActiveProjectId(firstId);
      localStorage.setItem(STORAGE_KEY, firstId);
    }
  }, [activeProjectId, projects]);

  // If the persisted ID no longer matches any project, clear it
  useEffect(() => {
    if (
      activeProjectId !== null &&
      projects &&
      !projects.some((p) => p.id === activeProjectId)
    ) {
      setActiveProjectId(null);
      localStorage.removeItem(STORAGE_KEY);
    }
  }, [activeProjectId, projects]);

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="sticky top-0 z-10 border-b bg-background/95 backdrop-blur px-6 py-3 flex items-center justify-between">
        <h1 className="text-lg font-semibold">Kulisawit</h1>
        <ProjectSwitcher
          activeProjectId={activeProjectId}
          onSelect={setActiveProjectId}
        />
      </header>
      <main className="p-6">
        {activeProjectId ? (
          <KanbanBoard projectId={activeProjectId} />
        ) : projects && projects.length === 0 ? (
          <EmptyState
            title="No kebun yet"
            description="Create one with curl -X POST /api/projects (until 3.2.2 ships the create form)."
          />
        ) : (
          <EmptyState
            title="Pick a kebun to view its board"
            description="Use the dropdown in the top-right corner."
          />
        )}
      </main>
    </div>
  );
}

export default App;

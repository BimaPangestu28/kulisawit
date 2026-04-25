import { useProjects } from "@/hooks/useProjects";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";

interface Props {
  activeProjectId: string | null;
  onSelect: (id: string) => void;
}

const STORAGE_KEY = "kulisawit.activeProject";

export function ProjectSwitcher({ activeProjectId, onSelect }: Props) {
  const { data, isLoading, isError } = useProjects();

  if (isLoading) {
    return <Skeleton data-testid="project-switcher-skeleton" className="h-9 w-48" />;
  }

  if (isError) {
    return (
      <div className="text-sm text-destructive" role="alert">
        Failed to load projects
      </div>
    );
  }

  if (!data || data.length === 0) {
    return (
      <Select disabled>
        <SelectTrigger className="w-48">
          <SelectValue placeholder="No projects" />
        </SelectTrigger>
      </Select>
    );
  }

  const handleChange = (value: string) => {
    localStorage.setItem(STORAGE_KEY, value);
    onSelect(value);
  };

  return (
    <Select value={activeProjectId ?? undefined} onValueChange={handleChange}>
      <SelectTrigger className="w-48">
        <SelectValue placeholder="Select a kebun" />
      </SelectTrigger>
      <SelectContent>
        {data.map((project) => (
          <SelectItem key={project.id} value={project.id}>
            {project.name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

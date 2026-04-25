import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { DndContext } from "@dnd-kit/core";
import { TaskCard } from "@/components/TaskCard";
import type { Task } from "@/types/api";

const baseTask: Task = {
  id: "t1",
  project_id: "p1",
  column_id: "c1",
  title: "Refactor parser",
  description: "Make it streaming",
  position: 0,
  tags: ["refactor"],
  linked_files: [],
  created_at: 0,
  updated_at: 0,
};

function renderWithDnd(ui: React.ReactElement) {
  return render(<DndContext>{ui}</DndContext>);
}

describe("TaskCard", () => {
  it("renders title and description", () => {
    renderWithDnd(<TaskCard task={baseTask} />);
    expect(screen.getByText("Refactor parser")).toBeInTheDocument();
    expect(screen.getByText("Make it streaming")).toBeInTheDocument();
  });

  it("hides description block when null", () => {
    const task: Task = { ...baseTask, description: null };
    renderWithDnd(<TaskCard task={task} />);
    expect(screen.getByText("Refactor parser")).toBeInTheDocument();
    expect(screen.queryByText("Make it streaming")).not.toBeInTheDocument();
  });
});

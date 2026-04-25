import { create } from "zustand";

interface UiState {
  selectedTaskId: string | null;
  isDetailOpen: boolean;
  expandedAttemptId: string | null;
  openDetail: (taskId: string) => void;
  closeDetail: () => void;
  expandAttempt: (id: string | null) => void;
}

export const useUiStore = create<UiState>((set) => ({
  selectedTaskId: null,
  isDetailOpen: false,
  expandedAttemptId: null,
  openDetail: (id) =>
    set({ selectedTaskId: id, isDetailOpen: true, expandedAttemptId: null }),
  closeDetail: () => set({ isDetailOpen: false, expandedAttemptId: null }),
  expandAttempt: (id) => set({ expandedAttemptId: id }),
}));

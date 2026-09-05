import { Boxes, Eye, ListTodo, type LucideIcon } from 'lucide-react';
import type { SessionKind } from '../ipc/ipc';

export const SESSION_KIND_ICON: Record<SessionKind, LucideIcon> = {
  task: ListTodo,
  explorer: Boxes,
  review: Eye,
};

export const SESSION_KIND_LABEL: Record<SessionKind, string> = {
  task: 'task',
  explorer: 'expl',
  review: 'review',
};

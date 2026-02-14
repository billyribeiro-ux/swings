export interface Course {
  id: string;
  title: string;
  level: 'Beginner' | 'Intermediate' | 'Advanced';
  description: string;
  meta: string;
  icon: string;
  gradient: { from: string; to: string };
}

export const courses: Course[] = [
  {
    id: 'beginning-options',
    title: 'Beginning to Options Trading',
    level: 'Beginner',
    description: 'Start from scratch. Learn what options are, how they work, and how to place your first trades with confidence — no prior experience needed.',
    meta: 'Self-Paced | All Levels',
    icon: 'BookOpen',
    gradient: { from: '#0B1D3A', to: '#1A3A6B' },
  },
  {
    id: 'options-101',
    title: 'Options Trading 101',
    level: 'Intermediate',
    description: 'Go deeper into calls, puts, spreads, and real strategies. Learn how to read the options chain, manage risk, and build consistent setups that work.',
    meta: 'Self-Paced | Intermediate',
    icon: 'Pulse',
    gradient: { from: '#1A3A6B', to: '#0FA4AF' },
  },
];

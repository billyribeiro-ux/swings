export interface Trader {
  id: string;
  name: string;
  initials: string;
  role: string;
  tagline: string;
  avatarGradient: { from: string; to: string };
  accentColor: string;
  bio: string[];
  highlights: { value: string; label: string }[];
  actions: { label: string; icon: string; variant: 'primary' | 'secondary' }[];
}

export const traders: Trader[] = [
  {
    id: 'billy',
    name: 'Billy Ribeiro',
    initials: 'BR',
    role: 'Founder & Lead Trader',
    tagline: 'Former lead trader at Simpler Trading. Creator of "The Move Prior to The Move" methodology. Mentored by Goldman Sachs.',
    avatarGradient: { from: '#0FA4AF', to: '#1A3A6B' },
    accentColor: '#0FA4AF',
    bio: [
      'Billy Ribeiro is a <strong>high-performance options trader</strong> known for his precision, discipline, and consistency. During his time at <strong>Simpler Trading</strong>, he quickly became their <strong>lead trader</strong> — at one point generating more winning trades in a single week than the entire staff combined.',
      'Mentored by <strong>Mark McGoldrick of Goldman Sachs</strong>, Billy developed his proprietary <strong>"Move Prior to The Move"</strong> methodology — a framework for identifying institutional-quality setups before the crowd catches on.',
      'After recovering from cancer, Billy shifted his focus toward sustainable, high-impact trading education. He now leads <strong>Explosive Swings</strong> and <strong>Revolution Trading Pros</strong>, serving over <strong>18,000 active traders</strong>.',
    ],
    highlights: [
      { value: '18K+', label: 'Active Traders' },
      { value: '600%', label: 'OXY Overnight' },
      { value: '573x', label: '0DTE SPX Return' },
    ],
    actions: [
      { label: 'Favorite Setups', icon: 'Star', variant: 'primary' },
      { label: 'Trading Style', icon: 'Pulse', variant: 'secondary' },
      { label: 'Notable Trades', icon: 'BookOpen', variant: 'secondary' },
    ],
  },
  {
    id: 'freddie',
    name: 'Freddie Ferber',
    initials: 'FF',
    role: 'Senior Trader',
    tagline: 'Disciplined swing trader with a sharp eye for high-probability setups and clean risk-to-reward entries.',
    avatarGradient: { from: '#D4A843', to: '#132B50' },
    accentColor: '#D4A843',
    bio: [
      'Freddie Ferber is a <strong>disciplined swing trader</strong> who specializes in identifying high-probability setups with clean risk-to-reward profiles. His methodical approach to the markets focuses on patience, precision, and letting the trade come to you.',
      'With deep expertise in <strong>technical analysis</strong> and <strong>price action</strong>, Freddie brings a complementary edge to the Explosive Swings watchlist — helping surface setups that meet the team\'s rigorous standards for quality and clarity.',
      'Freddie\'s trading philosophy centers on <strong>capital preservation first</strong> — every setup must have a defined risk before it earns a spot on the watchlist.',
    ],
    highlights: [
      { value: 'Swing', label: 'Primary Style' },
      { value: 'Stocks', label: 'Focus' },
      { value: 'R:R', label: 'Risk-First' },
    ],
    actions: [
      { label: 'Favorite Setups', icon: 'Star', variant: 'primary' },
      { label: 'Trading Style', icon: 'Pulse', variant: 'secondary' },
      { label: 'Notable Trades', icon: 'BookOpen', variant: 'secondary' },
    ],
  },
];

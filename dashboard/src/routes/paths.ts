// ----------------------------------------------------------------------

const ROOTS = {
  AUTH: '/auth',
  DASHBOARD: '/dashboard',
  CORTEX: '/cortex',
};

// ----------------------------------------------------------------------

export const paths = {
  faqs: '/faqs',
  minimalStore: 'https://mui.com/store/items/minimal-dashboard/',
  // AUTH
  auth: {
    amplify: {
      signIn: `${ROOTS.AUTH}/amplify/sign-in`,
      verify: `${ROOTS.AUTH}/amplify/verify`,
      signUp: `${ROOTS.AUTH}/amplify/sign-up`,
      updatePassword: `${ROOTS.AUTH}/amplify/update-password`,
      resetPassword: `${ROOTS.AUTH}/amplify/reset-password`,
    },
    jwt: {
      signIn: `${ROOTS.AUTH}/jwt/sign-in`,
    },
    firebase: {
      signIn: `${ROOTS.AUTH}/firebase/sign-in`,
      verify: `${ROOTS.AUTH}/firebase/verify`,
      signUp: `${ROOTS.AUTH}/firebase/sign-up`,
      resetPassword: `${ROOTS.AUTH}/firebase/reset-password`,
    },
    auth0: {
      signIn: `${ROOTS.AUTH}/auth0/sign-in`,
    },
    supabase: {
      signIn: `${ROOTS.AUTH}/supabase/sign-in`,
      verify: `${ROOTS.AUTH}/supabase/verify`,
      signUp: `${ROOTS.AUTH}/supabase/sign-up`,
      updatePassword: `${ROOTS.AUTH}/supabase/update-password`,
      resetPassword: `${ROOTS.AUTH}/supabase/reset-password`,
    },
  },
  // DASHBOARD
  dashboard: {
    root: ROOTS.DASHBOARD,
    // Multi-Agent System (Axon)
    agents: {
      root: `${ROOTS.DASHBOARD}/agents`,
      list: `${ROOTS.DASHBOARD}/agents`,
      create: `${ROOTS.DASHBOARD}/agents/create`,
      details: (id: string) => `${ROOTS.DASHBOARD}/agents/${id}`,
      byType: (type: string) => `${ROOTS.DASHBOARD}/agents/type/${type}`,
    },
    workflows: {
      root: `${ROOTS.DASHBOARD}/workflows`,
      list: `${ROOTS.DASHBOARD}/workflows`,
      create: `${ROOTS.DASHBOARD}/workflows/create`,
      templates: `${ROOTS.DASHBOARD}/workflows/templates`,
      details: (id: string) => `${ROOTS.DASHBOARD}/workflows/${id}`,
    },
    tasks: {
      root: `${ROOTS.DASHBOARD}/tasks`,
      list: `${ROOTS.DASHBOARD}/tasks`,
      details: (id: string) => `${ROOTS.DASHBOARD}/tasks/${id}`,
    },
    coordination: {
      root: `${ROOTS.DASHBOARD}/coordination`,
      messages: `${ROOTS.DASHBOARD}/coordination/messages`,
      sessions: `${ROOTS.DASHBOARD}/coordination/sessions`,
      locks: `${ROOTS.DASHBOARD}/coordination/locks`,
    },
    // Cognitive System (Cortex)
    memory: {
      root: `${ROOTS.DASHBOARD}/memory`,
      working: `${ROOTS.DASHBOARD}/memory/working`,
      episodic: `${ROOTS.DASHBOARD}/memory/episodic`,
      semantic: `${ROOTS.DASHBOARD}/memory/semantic`,
      patterns: `${ROOTS.DASHBOARD}/memory/patterns`,
      consolidation: `${ROOTS.DASHBOARD}/memory/consolidation`,
    },
    workspaces: {
      root: `${ROOTS.DASHBOARD}/workspaces`,
      list: `${ROOTS.DASHBOARD}/workspaces`,
      create: `${ROOTS.DASHBOARD}/workspaces/create`,
      details: (id: string) => `${ROOTS.DASHBOARD}/workspaces/${id}`,
      browse: (id: string) => `${ROOTS.DASHBOARD}/workspaces/${id}/browse`,
      codeUnits: (id: string) => `${ROOTS.DASHBOARD}/workspaces/${id}/code-units`,
      dependencies: (id: string) => `${ROOTS.DASHBOARD}/workspaces/${id}/dependencies`,
    },
    code: {
      root: `${ROOTS.DASHBOARD}/code`,
      analysis: `${ROOTS.DASHBOARD}/code/analysis`,
      dependencies: `${ROOTS.DASHBOARD}/code/dependencies`,
      quality: `${ROOTS.DASHBOARD}/code/quality`,
      vfs: `${ROOTS.DASHBOARD}/code/vfs`,
    },
    knowledge: {
      root: `${ROOTS.DASHBOARD}/knowledge`,
      documents: `${ROOTS.DASHBOARD}/knowledge/documents`,
      search: `${ROOTS.DASHBOARD}/knowledge/search`,
      createDocument: `${ROOTS.DASHBOARD}/knowledge/documents/create`,
      documentDetails: (id: string) => `${ROOTS.DASHBOARD}/knowledge/documents/${id}`,
    },
    // Monitoring & Analytics
    monitoring: {
      root: `${ROOTS.DASHBOARD}/monitoring`,
      metrics: `${ROOTS.DASHBOARD}/monitoring/metrics`,
      logs: `${ROOTS.DASHBOARD}/monitoring/logs`,
      health: `${ROOTS.DASHBOARD}/monitoring/health`,
    },
    // Configuration
    config: {
      root: `${ROOTS.DASHBOARD}/config`,
    },
  },
  // CORTEX - Cognitive System
  cortex: {
    root: ROOTS.CORTEX,
    overview: ROOTS.CORTEX,
    workspaces: {
      root: `${ROOTS.CORTEX}/workspaces`,
      list: `${ROOTS.CORTEX}/workspaces`,
      create: `${ROOTS.CORTEX}/workspaces/create`,
      details: (id: string) => `${ROOTS.CORTEX}/workspaces/${id}`,
      browse: (id: string) => `${ROOTS.CORTEX}/workspaces/${id}/browse`,
      codeUnits: (id: string) => `${ROOTS.CORTEX}/workspaces/${id}/code-units`,
      dependencies: (id: string) => `${ROOTS.CORTEX}/workspaces/${id}/dependencies`,
    },
    documents: {
      root: `${ROOTS.CORTEX}/documents`,
      list: `${ROOTS.CORTEX}/documents`,
      create: `${ROOTS.CORTEX}/documents/create`,
      view: (id: string) => `${ROOTS.CORTEX}/documents/${id}`,
    },
    memory: {
      root: `${ROOTS.CORTEX}/memory`,
      search: `${ROOTS.CORTEX}/memory`,
      episodes: `${ROOTS.CORTEX}/memory/episodes`,
      patterns: `${ROOTS.CORTEX}/memory/patterns`,
      consolidate: `${ROOTS.CORTEX}/memory/consolidate`,
    },
  },
};

# BlindMark Frontend

A standalone React/TypeScript demonstration workspace for the BlindMark project. Operations tables and charts use centralized demonstration fixtures; they are not live service metrics and do not imply authenticated coordination is active.

## Prerequisites

- Node.js 20.19+ or 22.12+ (Node 24 is also supported)
- npm 10+

## Install and run

```powershell
cd frontend
npm install
npm run dev
```

Vite prints the local URL. The application defaults to `http://127.0.0.1:5173`.

## Environment

Copy `.env.example` to `.env.local` and set `VITE_API_BASE_URL` to the BlindMark FastAPI base URL. The only live request currently implemented is an optional `GET /healthz` connection probe. No API credentials are stored by the frontend. The configured URL is a non-secret local preference and is not persisted to browser storage.

The Rust Phase 6E service is separate and has a different `/health` endpoint. No Rust service integration is implied by the FastAPI probe. Campaigns, tasks, nodes, journal records, activity, and security-control summaries shown in the workspace are demonstration fixtures until matching authenticated backend contracts exist.

## Checks and production build

```powershell
npm run typecheck
npm run lint
npm run test
npm run build
npm run preview
```

The production bundle is generated in `frontend/dist/`.

## Deployment

Deploy the contents of `dist/` to a static hosting provider. Configure SPA fallback/rewrite behavior so nested application routes serve `index.html`. Set `VITE_API_BASE_URL` at build time only if the health probe is needed. The frontend does not require the API to render its demo workspace.

## Future backend integration

The API client currently models only the confirmed FastAPI `/healthz` response. Before replacing demo adapters, agree and document typed endpoints, authentication boundaries, error semantics, pagination, and security properties with the backend owners. Do not treat demo records as verified service state.

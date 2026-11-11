# GUI R1 Interactive Prototypes

This is an isolated React + TypeScript + Vite prototype workspace. It is intentionally separate from the production `src/` tree and uses local mock data only.

## Run locally

```bash
cd docs/07-ui-design/gui-design-specification-r1/prototypes
npm install
npm run dev
```

Open the local Vite URL. Use the top prototype switcher to inspect:
1. Dashboard — metric filters, task completion and project navigation.
2. Project Workspace — view switching, task creation and project status preview.
3. Task Management — list/Kanban switching and task state transitions.
4. AI Assistant — mock suggestion and action preview; execution/approval remain disabled.

## Safety and fidelity

- No Tauri IPC imports.
- No network calls.
- No filesystem or credential access.
- No production data.
- No backend mutations.
- AI panel is explicitly deferred and preview-only.
- Sample values are fixture data, never production metrics.

These prototypes demonstrate interaction patterns, not backend integration or final visual sign-off.

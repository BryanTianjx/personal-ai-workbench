import { useState } from "react";
import { LayoutDashboard, Target, Layers3, FolderKanban, ListTodo, Search, Settings, Sparkles, PanelLeftClose, PanelLeftOpen, Bell, ChevronDown } from "lucide-react";
import Dashboard from "./Dashboard";
import ProjectWorkspace from "./ProjectWorkspace";
import TaskManagement from "./TaskManagement";
import AssistantPrototype from "./AssistantPrototype";

type Page = "dashboard" | "project" | "tasks" | "assistant";
const nav: { id: Page; label: string; icon: typeof LayoutDashboard }[] = [
  { id: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { id: "project", label: "Projects", icon: FolderKanban },
  { id: "tasks", label: "Tasks", icon: ListTodo },
  { id: "assistant", label: "AI Assistant · Preview", icon: Sparkles }
];

export default function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const [collapsed, setCollapsed] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [toast, setToast] = useState("");
  const notify = (message: string) => { setToast(message); window.setTimeout(() => setToast(""), 2200); };
  const title = nav.find(n => n.id === page)?.label ?? "Dashboard";

  return <div className="app-shell">
    <aside className={collapsed ? "sidebar collapsed" : "sidebar"}>
      <div className="brand"><div className="brand-mark">P</div>{!collapsed && <div><strong>Personal AI</strong><small>WORKBENCH</small></div>}</div>
      <div className="nav-label">WORKSPACE</div>
      {nav.filter(n => n.id !== "assistant").map(item => {
        const Icon = item.icon;
        return <button key={item.id} className={page === item.id ? "nav-item active" : "nav-item"} onClick={() => setPage(item.id)} title={item.label}><Icon size={17}/>{!collapsed && <span>{item.label}</span>}</button>;
      })}
      <div className="nav-label tools-label">TOOLS</div>
      <button className="nav-item" onClick={() => setSearchOpen(true)} title="Search"><Search size={17}/>{!collapsed && <span>Global Search</span>}<kbd>⌃K</kbd></button>
      <div className="sidebar-bottom">
        <button className={page === "assistant" ? "nav-item active" : "nav-item"} onClick={() => setPage("assistant")} title="Assistant preview"><Sparkles size={17}/>{!collapsed && <span>Assistant Preview</span>}</button>
        <button className="nav-item" onClick={() => notify("Settings prototype is not included in this review.")}><Settings size={17}/>{!collapsed && <span>Settings</span>}</button>
        <div className="workspace-user"><div className="avatar">B</div>{!collapsed && <div><strong>Personal Workspace</strong><small>Local workspace</small></div>}</div>
      </div>
      <button className="collapse-button" onClick={() => setCollapsed(!collapsed)}>{collapsed ? <PanelLeftOpen size={16}/> : <PanelLeftClose size={16}/>}</button>
    </aside>
    <main className="main-shell">
      <header className="topbar"><div className="crumb">Workspace <span>/</span> <strong>{title}</strong></div><div className="top-actions"><span className="local-status"><i/> Local</span><button className="icon-button" onClick={() => notify("No new notifications in this prototype.")}><Bell size={17}/></button><button className="profile-button" onClick={() => notify("Workspace menu is a visual prototype.")}>B <ChevronDown size={14}/></button></div></header>
      <div className="prototype-switcher"><span>R1 INTERACTIVE PROTOTYPES</span>{nav.map(n => <button key={n.id} className={page === n.id ? "switch active" : "switch"} onClick={() => setPage(n.id)}>{n.label.replace(" · Preview","")}</button>)}</div>
      <section className="page-area">
        {page === "dashboard" && <Dashboard onNavigate={setPage} notify={notify}/>}
        {page === "project" && <ProjectWorkspace notify={notify}/>}
        {page === "tasks" && <TaskManagement notify={notify}/>}
        {page === "assistant" && <AssistantPrototype notify={notify}/>}
      </section>
      <footer className="prototype-footer"><span>DESIGN PREVIEW · MOCK DATA ONLY</span><span>Not connected to production backend</span></footer>
    </main>
    {searchOpen && <div className="modal-backdrop" onMouseDown={e => { if (e.target === e.currentTarget) setSearchOpen(false); }}><div className="search-modal"><div className="search-input"><Search size={18}/><input autoFocus placeholder="Search projects, tasks and goals…" value={query} onChange={e => setQuery(e.target.value)} onKeyDown={e => e.key === "Escape" && setSearchOpen(false)}/><kbd>ESC</kbd></div><div className="search-results"><small>MOCK RESULTS</small>{(query ? ["Protocol Knowledge Base", "Review frontend design", "Define Alpha 0.1 acceptance"] : ["Recently opened", "Try searching for a task or project"]).map(x => <button key={x} onClick={() => {setSearchOpen(false);notify("Search result selected: " + x);}}>{x}</button>)}</div></div></div>}
    {toast && <div className="toast">{toast}</div>}
  </div>;
}

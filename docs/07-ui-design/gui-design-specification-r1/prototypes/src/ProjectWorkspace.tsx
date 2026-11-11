import { useState } from "react";
import { ArrowLeft, ArrowUpRight, CalendarDays, Check, ChevronDown, Circle, FileText, List, MoreHorizontal, Plus, Settings2, Users } from "lucide-react";

type Props = { notify: (message: string) => void };
type View = "Overview" | "List" | "Kanban" | "Activity";
const seed = [{id:1,title:"Define frontend architecture",status:"In progress",owner:"You",due:"Nov 13"},{id:2,title:"Review Design System tokens",status:"To do",owner:"You",due:"Nov 14"},{id:3,title:"Prepare GUI acceptance matrix",status:"Done",owner:"You",due:"Nov 10"}];

export default function ProjectWorkspace({notify}:Props) {
  const [view,setView] = useState<View>("Overview");
  const [tasks,setTasks] = useState(seed);
  const [showCreate,setShowCreate] = useState(false);
  const [draft,setDraft] = useState("");
  const addTask = () => { if(!draft.trim()) return; setTasks(t=>[...t,{id:Date.now(),title:draft.trim(),status:"To do",owner:"You",due:"Not set"}]); setDraft(""); setShowCreate(false); notify("Task added to local prototype data."); };
  return <div className="page-content">
    <div className="breadcrumb-line"><button className="back-link" onClick={()=>notify("Project list is represented by the navigation entry.")}><ArrowLeft size={14}/> Projects</button><span>/</span><span>Personal AI Workbench</span></div>
    <div className="project-title-row"><div><div className="eyebrow">PRODUCT · ACTIVE PROJECT</div><h1>Personal AI Workbench</h1><p>A local-first workspace for personal goals, projects and tasks.</p></div><div className="heading-actions"><button className="outline-button" onClick={()=>notify("Project settings are mock-only.")}><MoreHorizontal size={17}/> More</button><button className="button primary" onClick={()=>setShowCreate(true)}><Plus size={16}/> Add task</button></div></div>
    <div className="project-meta"><span className="status-pill active">● Active</span><span><Target size={14}/> Goal: Build a reliable personal workbench</span><span><CalendarDays size={14}/> Updated today</span><span><Users size={14}/> Personal workspace</span></div>
    <div className="project-tabs">{(["Overview","List","Kanban","Activity"] as View[]).map(v=><button key={v} className={view===v?"tab active":"tab"} onClick={()=>setView(v)}>{v==="List"?<List size={15}/>:v==="Activity"?<FileText size={15}/>:null}{v}</button>)}</div>
    {showCreate && <div className="inline-create"><input autoFocus value={draft} onChange={e=>setDraft(e.target.value)} placeholder="Write a task title…" onKeyDown={e=>e.key==="Enter"&&addTask()}/><button className="button primary small" onClick={addTask}>Create task</button><button className="text-button" onClick={()=>setShowCreate(false)}>Cancel</button></div>}
    {view==="Overview" && <div className="project-overview">
      <div className="overview-main">
        <section className="panel"><div className="panel-heading"><div><h2>Project progress</h2><p>Illustrative progress summary</p></div><span className="progress-big">56%</span></div><div className="progress-track large"><i style={{width:"56%"}}/></div><div className="progress-foot"><span>5 tasks completed</span><span>4 remaining</span></div></section>
        <section className="panel"><div className="panel-heading"><div><h2>Recent tasks</h2><p>Latest project work items</p></div><button className="text-button" onClick={()=>setView("List")}>View all <ArrowUpRight size={14}/></button></div>{tasks.slice(0,3).map(t=><TaskLine key={t.id} task={t}/>)}</section>
      </div>
      <div className="overview-side"><section className="panel"><h2>Project details</h2><p className="detail-text">This project demonstrates the core workspace experience, including product requirements, frontend design and engineering acceptance.</p><div className="detail-pair"><span>Owner</span><strong>You</strong></div><div className="detail-pair"><span>Priority</span><strong>High</strong></div><div className="detail-pair"><span>Target</span><strong>Alpha 0.1</strong></div></section><section className="panel"><h2>Resources</h2><button className="resource-link" onClick={()=>notify("Attachments are not connected in the prototype.")}><FileText size={16}/> Project documents <ArrowUpRight size={13}/></button></section></div>
    </div>}
    {view==="List" && <section className="panel"><div className="panel-heading"><div><h2>Project tasks</h2><p>{tasks.length} tasks in this prototype</p></div><button className="outline-button" onClick={()=>setShowCreate(true)}><Plus size={15}/> Add task</button></div><div className="table-head"><span>TASK</span><span>STATUS</span><span>OWNER</span><span>DUE DATE</span></div>{tasks.map(t=><div className="table-row" key={t.id}><strong>{t.title}</strong><span className={"task-status "+(t.status==="Done"?"done":t.status==="In progress"?"progress":"todo")}>{t.status}</span><span>{t.owner}</span><span>{t.due}</span></div>)}</section>}
    {view==="Kanban" && <div className="kanban-grid">{["To do","In progress","Done"].map(s=><div className="kanban-column" key={s}><div className="column-title"><strong>{s}</strong><span>{tasks.filter(t=>t.status===s).length}</span></div>{tasks.filter(t=>t.status===s).map(t=><div className="kanban-card" key={t.id}><strong>{t.title}</strong><small>{t.due} · {t.owner}</small><select value={t.status} onChange={e=>setTasks(all=>all.map(x=>x.id===t.id?{...x,status:e.target.value}:x))}><option>To do</option><option>In progress</option><option>Done</option></select></div>)}</div>)}</div>}
    {view==="Activity" && <section className="panel"><h2>Recent activity</h2><div className="activity-line"><i/> <div><strong>Project opened for design review</strong><small>Today · Prototype fixture</small></div></div><div className="activity-line"><i/> <div><strong>Task list updated</strong><small>Yesterday · Prototype fixture</small></div></div></section>}
    <p className="fixture-note">Project, task and progress data are local fixtures. No production IPC calls are made.</p>
  </div>;
}
function TaskLine({task}:{task:{id:number;title:string;status:string;owner:string;due:string}}){return <div className="task-row"><span className={task.status==="Done"?"check done":"check"}>{task.status==="Done"?<Check size={13}/>:null}</span><div className="task-copy"><strong>{task.title}</strong><small>{task.due}</small></div><span className={"task-status "+(task.status==="Done"?"done":task.status==="In progress"?"progress":"todo")}>{task.status}</span></div>;}

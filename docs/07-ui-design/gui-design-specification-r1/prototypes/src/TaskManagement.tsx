import { useMemo, useState } from "react";
import { Check, Circle, Filter, List, Plus, Search, SlidersHorizontal } from "lucide-react";

type Props={notify:(message:string)=>void};
type Status="To do"|"In progress"|"Done";
type Task={id:number;title:string;project:string;priority:"High"|"Medium"|"Low";status:Status;due:string};
const initial:Task[]=[
{id:1,title:"Review frontend architecture",project:"Personal AI Workbench",priority:"High",status:"In progress",due:"Nov 13"},
{id:2,title:"Finalize Design System tokens",project:"Personal AI Workbench",priority:"Medium",status:"To do",due:"Nov 14"},
{id:3,title:"Verify IPC command contracts",project:"Personal AI Workbench",priority:"High",status:"To do",due:"Nov 15"},
{id:4,title:"Prepare protocol notes",project:"Protocol Knowledge",priority:"Low",status:"Done",due:"Nov 10"},
{id:5,title:"Review operation lifecycle",project:"FEMRIX",priority:"Medium",status:"In progress",due:"Nov 18"}
];

export default function TaskManagement({notify}:Props){
 const [tasks,setTasks]=useState(initial);
 const [mode,setMode]=useState<"List"|"Kanban">("Kanban");
 const [filter,setFilter]=useState("All");
 const [search,setSearch]=useState("");
 const filtered=useMemo(()=>tasks.filter(t=>(filter==="All"||t.priority===filter)&&t.title.toLowerCase().includes(search.toLowerCase())),[tasks,filter,search]);
 const move=(id:number,status:Status)=>setTasks(all=>all.map(t=>t.id===id?{...t,status}:t));
 return <div className="page-content">
  <div className="page-heading"><div><div className="eyebrow">WORK MANAGEMENT</div><h1>Tasks</h1><p>Track the next actions across your active projects.</p></div><button className="button primary" onClick={()=>notify("Task creation form is not connected in this prototype.")}><Plus size={16}/> New task</button></div>
  <div className="task-toolbar"><div className="search-field"><Search size={16}/><input value={search} onChange={e=>setSearch(e.target.value)} placeholder="Filter tasks…"/></div><label className="select-filter"><Filter size={15}/><select value={filter} onChange={e=>setFilter(e.target.value)}><option>All</option><option>High</option><option>Medium</option><option>Low</option></select></label><div className="view-toggle"><button className={mode==="List"?"selected":""} onClick={()=>setMode("List")}><List size={15}/> List</button><button className={mode==="Kanban"?"selected":""} onClick={()=>setMode("Kanban")}><SlidersHorizontal size={15}/> Board</button></div></div>
  {mode==="Kanban"?<div className="kanban-grid">{(["To do","In progress","Done"] as Status[]).map(status=><div className="kanban-column" key={status}><div className="column-title"><strong>{status}</strong><span>{filtered.filter(t=>t.status===status).length}</span></div>{filtered.filter(t=>t.status===status).map(t=><article className="kanban-card" key={t.id}><div className="card-top"><span className={"priority "+t.priority.toLowerCase()}>{t.priority}</span><button className="mini-dots" onClick={()=>notify("Task actions are preview-only.")}>···</button></div><strong>{t.title}</strong><small>{t.project}</small><div className="kanban-card-foot"><span>{t.due}</span><select aria-label="Change task status" value={t.status} onChange={e=>move(t.id,e.target.value as Status)}><option>To do</option><option>In progress</option><option>Done</option></select></div></article>)}</div>)}</div>
  :<section className="panel"><div className="table-head task-table-head"><span>TASK</span><span>PROJECT</span><span>PRIORITY</span><span>STATUS</span><span>DUE</span></div>{filtered.map(t=><div className="table-row task-table-row" key={t.id}><div className="task-table-title"><span className={t.status==="Done"?"check done":"check"}>{t.status==="Done"?<Check size={12}/>:null}</span><strong>{t.title}</strong></div><span>{t.project}</span><span className={"priority "+t.priority.toLowerCase()}>{t.priority}</span><select value={t.status} onChange={e=>move(t.id,e.target.value as Status)}><option>To do</option><option>In progress</option><option>Done</option></select><span>{t.due}</span></div>)}</section>}
  {filtered.length===0&&<div className="empty-state"><Circle size={24}/><strong>No matching tasks</strong><span>Try another search or priority filter.</span></div>}
  <p className="fixture-note">Changing status updates local prototype state only. Production transitions must be validated by the Rust domain/application layer.</p>
 </div>;
}

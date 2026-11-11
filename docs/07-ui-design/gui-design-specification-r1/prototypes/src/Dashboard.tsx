import { useState, type ReactNode } from "react";
import { ArrowUpRight, Check, Circle, Clock3, FolderKanban, MoreHorizontal, Plus, Target, TrendingUp } from "lucide-react";

type Props = { onNavigate: (page: "dashboard" | "project" | "tasks" | "assistant") => void; notify: (message: string) => void };
const initialTasks = [
  { id: 1, title: "Review frontend architecture", meta: "Personal AI Workbench", done: false, priority: "High" },
  { id: 2, title: "Finalize Alpha 0.1 acceptance criteria", meta: "Personal AI Workbench", done: false, priority: "Medium" },
  { id: 3, title: "整理协议知识库文档", meta: "Protocol Knowledge", done: true, priority: "Low" }
];

export default function Dashboard({ onNavigate, notify }: Props) {
  const [tasks, setTasks] = useState(initialTasks);
  const [filter, setFilter] = useState<"all" | "open" | "done">("all");
  const visible = tasks.filter(t => filter === "all" || (filter === "done" ? t.done : !t.done));
  const toggle = (id: number) => setTasks(items => items.map(t => t.id === id ? { ...t, done: !t.done } : t));
  return <div className="page-content">
    <div className="page-heading"><div><div className="eyebrow">WEDNESDAY, NOVEMBER 11</div><h1>Good evening, Bryan</h1><p>Here’s what is happening in your workspace today.</p></div><button className="button primary" onClick={() => notify("Quick-create is a prototype interaction.")}><Plus size={16}/> Quick capture</button></div>
    <div className="metric-grid">
      <Metric label="Open tasks" value={String(tasks.filter(t => !t.done).length + 5).padStart(2,"0")} note="Across active projects" icon={<ListIcon/>}/>
      <Metric label="In progress" value="04" note="2 due this week" icon={<Clock3/>}/>
      <Metric label="Active projects" value="06" note="Across 3 areas" icon={<FolderKanban/>}/>
      <Metric label="Goals on track" value="03" note="1 needs attention" icon={<Target/>}/>
    </div>
    <div className="dashboard-grid">
      <section className="panel task-panel">
        <div className="panel-heading"><div><h2>Today’s focus</h2><p>Keep momentum on the work that matters.</p></div><button className="text-button" onClick={() => onNavigate("tasks")}>View all <ArrowUpRight size={14}/></button></div>
        <div className="filter-row">{(["all","open","done"] as const).map(f => <button key={f} className={filter === f ? "filter active" : "filter"} onClick={() => setFilter(f)}>{f === "all" ? "All" : f === "open" ? "To do" : "Completed"}</button>)}</div>
        <div className="task-list">{visible.map(task => <div className="task-row" key={task.id}><button className={task.done ? "check done" : "check"} onClick={() => toggle(task.id)} aria-label="Toggle task">{task.done && <Check size={13}/>}</button><div className="task-copy"><strong className={task.done ? "strike" : ""}>{task.title}</strong><small>{task.meta}</small></div><span className={"priority " + task.priority.toLowerCase()}>{task.priority}</span><button className="icon-button subtle" onClick={() => notify("Task detail preview: " + task.title)}><MoreHorizontal size={17}/></button></div>)}</div>
        {visible.length === 0 && <div className="empty-inline">No tasks in this view.</div>}
      </section>
      <section className="panel goals-panel">
        <div className="panel-heading"><div><h2>Goals in focus</h2><p>Progress from local workspace fixtures.</p></div><button className="icon-button subtle" onClick={() => notify("Goal actions are not connected.")}><MoreHorizontal size={17}/></button></div>
        <Goal title="Build a reliable personal workbench" progress={68} color="blue" detail="3 of 5 milestones"/>
        <Goal title="Improve protocol knowledge workflow" progress={42} color="violet" detail="2 of 6 milestones"/>
        <button className="outline-button full" onClick={() => notify("Goal list navigation is not part of this prototype.")}>Explore goals <ArrowUpRight size={14}/></button>
      </section>
      <section className="panel projects-panel">
        <div className="panel-heading"><div><h2>Recent projects</h2><p>Pick up where you left off.</p></div><button className="text-button" onClick={() => onNavigate("project")}>All projects <ArrowUpRight size={14}/></button></div>
        <ProjectRow name="Personal AI Workbench" tag="Product" status="Active" progress={56}/>
        <ProjectRow name="Protocol Knowledge" tag="Engineering" status="Active" progress={73}/>
        <ProjectRow name="FEMRIX Architecture" tag="Research" status="Paused" progress={31}/>
      </section>
      <section className="panel activity-panel">
        <div className="panel-heading"><div><h2>Workspace pulse</h2><p>Illustrative activity summary.</p></div><TrendingUp size={18} className="muted-icon"/></div>
        <div className="pulse-number">12 <span>updates this week</span></div>
        <div className="pulse-bars">{[30,54,42,72,50,88,63].map((h,i)=><i key={i} style={{height:h+"%"}}/>)}</div>
        <div className="pulse-legend"><span>Mon</span><span>Sun</span></div>
      </section>
    </div>
    <p className="fixture-note">All counts and progress values are sample fixtures for visual review, not live workspace data.</p>
  </div>;
}
function Metric({label,value,note,icon}:{label:string;value:string;note:string;icon:ReactNode}) { return <div className="metric-card"><div className="metric-top"><span>{label}</span><i>{icon}</i></div><strong>{value}</strong><small>{note}</small></div>; }
function Goal({title,progress,color,detail}:{title:string;progress:number;color:string;detail:string}) { return <div className="goal-item"><div className="goal-title"><strong>{title}</strong><span>{progress}%</span></div><div className="progress-track"><i className={color} style={{width:progress+"%"}}/></div><small>{detail}</small></div>; }
function ProjectRow({name,tag,status,progress}:{name:string;tag:string;status:string;progress:number}) { return <div className="project-row"><div className="project-symbol"><FolderKanban size={17}/></div><div className="project-info"><strong>{name}</strong><small>{tag} · {status}</small></div><div className="project-progress"><div className="progress-track"><i style={{width:progress+"%"}}/></div><small>{progress}%</small></div></div>; }
function ListIcon(){return <Circle/>;}

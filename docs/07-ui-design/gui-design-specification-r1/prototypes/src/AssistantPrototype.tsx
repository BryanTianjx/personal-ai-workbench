import { useState } from "react";
import { AlertTriangle, ArrowRight, Check, ChevronDown, CircleHelp, LockKeyhole, Plus, Send, ShieldCheck, Sparkles, WandSparkles } from "lucide-react";

type Props={notify:(message:string)=>void};
export default function AssistantPrototype({notify}:Props){
 const [prompt,setPrompt]=useState("Help me break down the frontend design review into actionable steps.");
 const [preview,setPreview]=useState(false);
 const [context,setContext]=useState(true);
 const [messages,setMessages]=useState<string[]>([]);
 const suggest=()=>{if(!prompt.trim())return;setMessages(m=>[...m,prompt.trim()]);setPrompt("");setPreview(true);};
 return <div className="page-content assistant-page">
  <div className="page-heading"><div><div className="eyebrow">FUTURE CAPABILITY · MOCK ONLY</div><h1>AI Assistant</h1><p>Context-aware assistance with explicit review before any proposed action.</p></div><span className="deferred-pill"><CircleHelp size={14}/> Not enabled in Alpha 0.1</span></div>
  <div className="assistant-layout">
   <section className="assistant-chat panel">
    <div className="assistant-header"><div className="assistant-avatar"><Sparkles size={18}/></div><div><strong>Workbench Assistant</strong><small><i/> Prototype conversation · no provider connected</small></div><button className="icon-button subtle" onClick={()=>notify("Assistant settings are not implemented.")}><ChevronDown size={17}/></button></div>
    <div className="chat-scroll">
     <div className="assistant-message"><div className="assistant-avatar small"><Sparkles size={14}/></div><div className="bubble"><strong>How can I help with your work?</strong><p>I can help organize ideas, review a project plan or draft task breakdowns. In this prototype, suggestions are illustrative and do not access your workspace.</p><div className="suggestion-chips"><button onClick={()=>setPrompt("Summarize the project risks")}>Summarize project risks</button><button onClick={()=>setPrompt("Break a goal into milestones")}>Break goal into milestones</button></div></div></div>
     {messages.map((m,i)=><div className="user-message" key={i}>{m}</div>)}
     {preview&&<div className="assistant-message"><div className="assistant-avatar small"><Sparkles size={14}/></div><div className="bubble"><strong>Suggested breakdown</strong><p>Here is a possible sequence for review:</p><ol><li>Confirm page scope and acceptance criteria</li><li>Review Design System tokens and shared components</li><li>Validate IPC contracts against Rust handlers</li><li>Run visual and interaction acceptance tests</li></ol><span className="mock-label">MOCK SUGGESTION · NOT SAVED</span></div></div>}
    </div>
    <div className="composer"><textarea value={prompt} onChange={e=>setPrompt(e.target.value)} placeholder="Ask about your work…" rows={2}/><div className="composer-foot"><span>Enter to send · Shift+Enter for newline</span><button className="button primary" onClick={suggest}><Send size={15}/> Send</button></div></div>
   </section>
   <aside className="assistant-side">
    <section className="panel context-card"><div className="panel-heading"><div><h2>Context</h2><p>Explicitly selected information</p></div><button className="toggle" onClick={()=>setContext(!context)}>{context?"On":"Off"}</button></div>{context?<><div className="context-item"><span className="context-icon"><CircleHelp size={15}/></span><div><strong>Current page</strong><small>GUI Design R1 prototype</small></div><Check size={14} className="success-icon"/></div><div className="context-item"><span className="context-icon"><FolderIcon/></span><div><strong>Workspace</strong><small>Personal workspace · sample</small></div><Check size={14} className="success-icon"/></div></>:<div className="muted-box">No context selected.</div>}<p className="context-disclaimer">Future production context must be inspectable and scoped. No real records are attached here.</p></section>
    <section className="panel action-preview"><div className="panel-heading"><div><h2>Action preview</h2><p>Review before execution</p></div><LockKeyhole size={16} className="muted-icon"/></div><div className="preview-status"><AlertTriangle size={15}/> Preview only — no action can run</div><div className="preview-operation"><span className="operation-icon"><WandSparkles size={16}/></span><div><strong>Create 4 draft tasks</strong><small>Target: Personal AI Workbench</small></div></div><ul><li>4 proposed task records</li><li>No files or external services</li><li>Would require backend authorization</li></ul><button className="button primary full" disabled><ShieldCheck size={15}/> Approval unavailable</button><small className="disabled-note">Execution is deferred. A UI confirmation must never grant capabilities.</small></section>
   </aside>
  </div>
  <p className="fixture-note">This screen is a future-facing interaction study. It has no model, adapter, IPC, authorization or persistence integration.</p>
 </div>;
}
function FolderIcon(){return <Plus size={15}/>;}

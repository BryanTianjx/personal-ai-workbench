// E2 vertical slice UI: the Work Management experience end to end —
// workspace picker (persisted selection), real-data Dashboard (PAW-F-004),
// Area/Goal/Project/Task management with typed status transitions, scoped
// search (E2-03) and task attachments (E2-04). This screen talks only to
// paw-desktop commands; it never touches storage directly (ADR-018).

import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// ---- Serialized contract shapes (snake_case across the boundary) ----

type Workspace = { id: string; name: string };
type Area = { id: string; workspace_id: string; name: string; description: string | null };
type Goal = { id: string; workspace_id: string; area_id: string | null; name: string; description: string | null };
type Project = {
  id: string;
  workspace_id: string;
  goal_id: string | null;
  area_id: string | null;
  name: string;
  description: string | null;
  status: "active" | "paused" | "completed" | "archived";
};
type Task = {
  id: string;
  workspace_id: string;
  project_id: string | null;
  title: string;
  notes: string | null;
  status: "todo" | "in_progress" | "done" | "cancelled";
};
type Attachment = {
  id: string;
  workspace_id: string;
  owner_type: "project" | "task";
  owner_id: string;
  managed_id: string;
  file_name: string;
  size_bytes: number;
  sha256: string;
  created_at: number;
};
type SearchHit = { kind: "project" | "task"; id: string; workspace_id: string; title: string };
type Dashboard = {
  workspace_id: string;
  area_count: number;
  goal_count: number;
  projects_by_status: [string, number][];
  tasks_by_status: [string, number][];
  attachment_count: number;
  roadmap_count: number;
  learning_plan_count: number;
  learning_item_count: number;
  learning_session_count: number;
};
type Roadmap = {
  id: string;
  workspace_id: string;
  name: string;
  description: string | null;
};
type Plan = {
  id: string;
  workspace_id: string;
  roadmap_id: string;
  name: string;
  description: string | null;
  status: "draft" | "active" | "paused" | "completed" | "archived";
};
type LearningItem = {
  id: string;
  workspace_id: string;
  plan_id: string;
  kind:
    | "concept"
    | "reading"
    | "exercise"
    | "experiment"
    | "implementation"
    | "review";
  title: string;
  expected_outcome: string | null;
  acceptance_criteria: string | null;
  notes: string | null;
  status: "todo" | "in_progress" | "done" | "cancelled";
};
type LearningSession = {
  id: string;
  workspace_id: string;
  item_id: string;
  task_id: string | null;
  project_id: string | null;
  method: string | null;
  duration_minutes: number | null;
  summary: string | null;
  difficulty: string | null;
  takeaway: string | null;
  next_step: string | null;
  occurred_at: number;
};
type ProjectRetrospective = {
  id: string;
  workspace_id: string;
  project_id: string;
  summary: string;
  learning_session_refs: string[];
};
type KnowledgeRef = {
  id: string;
  workspace_id: string;
  title: string;
  source: "lingshu" | "hindsight" | "web" | "local";
  external_uri: string | null;
  locator: string | null;
  version_label: string | null;
  notes: string | null;
  availability: "available" | "stale" | "unavailable";
};
type OperationRow = {
  id: string;
  kind: string;
  workspace_id: string | null;
  status:
    | "pending"
    | "running"
    | "awaiting_approval"
    | "retry_scheduled"
    | "outcome_unknown"
    | "succeeded"
    | "failed"
    | "cancelled";
  next_attempt_at: number | null;
  cancellation_requested_at: number | null;
};
type QuarantinedMessage = {
  id: string;
  event_type: string;
  attempts: number;
  redrive_count: number;
  last_error_category: string | null;
  last_redrive_by: string | null;
};

type AgentRegistration = {
  id: string;
  workspace_id: string;
  display_name: string;
  adapter_kind: "fake" | "cli_hermes" | "cli_codex" | "api_custom";
  declared_capabilities: string[];
  status: "Active" | "Suspended" | "Retired";
};

type AgentExecution = {
  id: string;
  registration_id: string;
  workspace_id: string;
  capability_id: string;
  capability_contract_version: string;
  plan_digest: string;
  tool_adapter: string;
  target_identity: string;
  material_params_digest: string;
  parameters_json: string;
  status:
    | "Created"
    | "Validating"
    | "AwaitingApproval"
    | "Approved"
    | "Submitted"
    | "Running"
    | "Completed"
    | "Failed"
    | "Cancelled"
    | "TimedOut"
    | "OutcomeUnknown";
  operation_id: string | null;
};

type ApprovalDisplay = {
  execution_id: string;
  capability_id: string;
  capability_contract_version: string;
  tool_adapter: string;
  target_identity: string;
  side_effect_set: string[];
  risk_class: string;
  expires_at_ms: number;
  attempt_policy: string;
};

type AgentProposalOutcome = {
  status: "completed" | "awaiting_approval";
  execution: AgentExecution;
  operation: { id: string; status: string; correlation_id: string } | null;
  display: ApprovalDisplay | null;
  summary: string | null;
};

// The pending approval the UI can decide on (one at a time in the E5
// slice; the decision surface shows target/action/risk/expiry).
type PendingApproval = {
  executionId: string;
  display: ApprovalDisplay;
};

type AgentRunOutcome = {
  status: "completed" | "failed" | "unresolved";
  execution: AgentExecution;
  operation: { id: string; status: string; correlation_id: string };
  summary: string;
};

// E6: skill matrix / learning gaps / creator pipeline (serialized shapes).
type Skill = {
  id: string;
  workspace_id: string;
  name: string;
  category: string;
  scale_version: number;
  notes: string | null;
};
type LearningGapProposal = {
  id: string;
  workspace_id: string;
  project_id: string;
  gap: string;
  status: "Proposed" | "Confirmed" | "Dismissed";
  confirmed_item_id: string | null;
};
type CreatorTopic = {
  id: string;
  workspace_id: string;
  title: string;
  notes: string | null;
  status: "Idea" | "Selected" | "Parked";
};
type CreatorDraft = {
  id: string;
  workspace_id: string;
  topic_id: string;
  title: string;
  current_version_no: number;
  status: "Writing" | "InReview" | "Approved" | "Published";
};
type CreatorDraftReview = {
  id: string;
  draft_id: string;
  version_no: number;
  decision: "Approved" | "Rejected";
  comment: string | null;
};

type CommandError =
  | "empty_name"
  | "name_too_long"
  | "notes_too_long"
  | "workspace_not_found"
  | "project_not_found"
  | "area_not_found"
  | "goal_not_found"
  | "task_not_found"
  | "invalid_workspace_id"
  | "task_status_invalid"
  | "invalid_search_query"
  | "attachment_not_found"
  | "owner_not_found"
  | "denied"
  | "unexpected";

const ERROR_TEXT: Record<CommandError, string> = {
  empty_name: "名称不能为空。",
  name_too_long: "名称超出长度限制。",
  notes_too_long: "备注超出长度限制。",
  workspace_not_found: "工作空间不存在。",
  project_not_found: "项目不存在。",
  area_not_found: "领域不存在。",
  goal_not_found: "目标不存在。",
  task_not_found: "任务不存在。",
  invalid_workspace_id: "工作空间标识格式无效。",
  task_status_invalid: "该状态变更不被允许。",
  invalid_search_query: "搜索词无效。",
  attachment_not_found: "附件不存在。",
  owner_not_found: "附件所属记录不存在。",
  denied: "操作被拒绝。",
  unexpected: "操作失败，请重试。",
};

function describe(e: unknown): string {
  if (typeof e === "string" && e in ERROR_TEXT) {
    return ERROR_TEXT[e as CommandError];
  }
  return ERROR_TEXT.unexpected;
}

const WORKSPACE_KEY = "paw.workspace-id";

export default function App() {
  // ---- Workspace state ----
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [workspaceName, setWorkspaceName] = useState("");
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // ---- Record lists ----
  const [areas, setAreas] = useState<Area[]>([]);
  const [goals, setGoals] = useState<Goal[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [dashboard, setDashboard] = useState<Dashboard | null>(null);

  // ---- Form inputs ----
  const [areaName, setAreaName] = useState("");
  const [goalName, setGoalName] = useState("");
  const [goalArea, setGoalArea] = useState<string>("");
  const [projectName, setProjectName] = useState("");
  const [projectGoal, setProjectGoal] = useState<string>("");
  const [projectArea, setProjectArea] = useState<string>("");
  const [taskTitle, setTaskTitle] = useState("");
  const [taskProject, setTaskProject] = useState<string>("");
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<SearchHit[] | null>(null);
  const [taskAttachments, setTaskAttachments] = useState<Record<string, Attachment[]>>({});
  const [operations, setOperations] = useState<OperationRow[]>([]);
  const [quarantined, setQuarantined] = useState<QuarantinedMessage[]>([]);

  // ---- Learning & Knowledge state (E4) ----
  const [roadmaps, setRoadmaps] = useState<Roadmap[]>([]);
  const [plans, setPlans] = useState<Plan[]>([]);
  const [learningItems, setLearningItems] = useState<LearningItem[]>([]);
  const [learningSessions, setLearningSessions] = useState<LearningSession[]>([]);
  const [knowledgeRefs, setKnowledgeRefs] = useState<KnowledgeRef[]>([]);
  const [roadmapName, setRoadmapName] = useState("");
  const [planName, setPlanName] = useState("");

  const [itemTitle, setItemTitle] = useState("");
  const [itemKind, setItemKind] = useState<LearningItem["kind"]>("implementation");


  const [sessionSummary, setSessionSummary] = useState("");
  const [sessionItem, setSessionItem] = useState<string>("");
  const [knowledgeTitle, setKnowledgeTitle] = useState("");
  const [knowledgeSource, setKnowledgeSource] = useState<KnowledgeRef["source"]>("web");
  const [knowledgeUri, setKnowledgeUri] = useState("");

  // Agent governance (E5): registrations, executions, the pending approval
  // surface, and the proposal form state.
  const [agents, setAgents] = useState<AgentRegistration[]>([]);
  const [agentExecutions, setAgentExecutions] = useState<AgentExecution[]>([]);
  const [pendingApproval, setPendingApproval] = useState<PendingApproval | null>(null);
  const [agentName, setAgentName] = useState("");
  const [propAgent, setPropAgent] = useState<string>("");
  const [propCapability, setPropCapability] = useState("work_management.task.create");

  // E6: skills / learning gaps / creator pipeline state.
  const [skills, setSkills] = useState<Skill[]>([]);
  const [learningGaps, setLearningGaps] = useState<LearningGapProposal[]>([]);
  const [creatorTopics, setCreatorTopics] = useState<CreatorTopic[]>([]);
  const [creatorDrafts, setCreatorDrafts] = useState<CreatorDraft[]>([]);
  const [skillName, setSkillName] = useState("");
  const [skillCategory, setSkillCategory] = useState("");
  const [gapProject, setGapProject] = useState<string>("");
  const [gapText, setGapText] = useState("");
  const [gapItem, setGapItem] = useState<string>("");
  const [topicTitle, setTopicTitle] = useState("");
  const [draftTitle, setDraftTitle] = useState("");
  const [draftBody, setDraftBody] = useState("");
  // E6 第二批: publish records + project retrospectives.
  const [retroProject, setRetroProject] = useState<string>("");
  const [retroSummary, setRetroSummary] = useState("");
  const [retroSession, setRetroSession] = useState<string>("");
  const [retrospectives, setRetrospectives] = useState<ProjectRetrospective[]>([]);

  const fail = (e: unknown) => setNotice(describe(e));

  // ---- Loaders ----
  const reloadWorkspaces = useCallback(() => {
    invoke<Workspace[]>("list_workspaces")
      .then((list) => {
        setWorkspaces(list);
        setWorkspaceId((current) => {
          if (current && list.some((w) => w.id === current)) return current;
          const saved = localStorage.getItem(WORKSPACE_KEY);
          if (saved && list.some((w) => w.id === saved)) return saved;
          return list[0]?.id ?? null;
        });
      })
      .catch(fail);
  }, []);

  const reloadAll = useCallback(() => {
    if (!workspaceId) {
      setAreas([]);
      setGoals([]);
      setProjects([]);
      setTasks([]);
      setDashboard(null);
      setTaskAttachments({});
      return;
    }
    localStorage.setItem(WORKSPACE_KEY, workspaceId);
    invoke<Area[]>("list_areas", { workspaceId })
      .then((r) => setAreas(r))
      .catch(fail);
    invoke<Goal[]>("list_goals", { workspaceId, areaId: null })
      .then((r) => setGoals(r))
      .catch(fail);
    invoke<Project[]>("list_projects", { workspaceId })
      .then((r) => setProjects(r))
      .catch(fail);
    invoke<Task[]>("list_tasks", { workspaceId, projectId: null, status: null })
      .then((r) => setTasks(r))
      .catch(fail);
    invoke<Dashboard>("dashboard", { workspaceId })
      .then((r) => setDashboard(r))
      .catch(fail);
    // Attachment lists per task (metadata only).
    invoke<Task[]>("list_tasks", { workspaceId, projectId: null, status: null })
      .then((list) =>
        Promise.all(
          list.map((t) =>
            invoke<Attachment[]>("list_attachments", {
              workspaceId,
              ownerType: "task",
              ownerId: t.id,
            }).then((atts) => [t.id, atts] as const),
          ),
        ),
      )
      .then((pairs) => setTaskAttachments(Object.fromEntries(pairs)))
      .catch(fail);
    // Operation queue view (E3-04): workspace-scoped operations + poison queue.
    invoke<OperationRow[]>("list_operations", { workspaceId })
      .then((r) => setOperations(r))
      .catch(fail);
    invoke<QuarantinedMessage[]>("list_quarantined_messages", { workspaceId })
      .then((r) => setQuarantined(r))
      .catch(fail);
    // Learning & Knowledge lists (E4).
    invoke<Roadmap[]>("list_roadmaps", { workspaceId })
      .then((r) => setRoadmaps(r))
      .catch(fail);
    invoke<Plan[]>("list_plans", { workspaceId, roadmapId: null })
      .then((r) => setPlans(r))
      .catch(fail);
    invoke<LearningItem[]>("list_items", { workspaceId, planId: null })
      .then((r) => setLearningItems(r))
      .catch(fail);
    invoke<LearningSession[]>("list_sessions", { workspaceId, itemId: null })
      .then((r) => setLearningSessions(r))
      .catch(fail);
    invoke<KnowledgeRef[]>("list_knowledge_refs", { workspaceId, availability: null })
      .then((r) => setKnowledgeRefs(r))
      .catch(fail);
    // Agent governance lists (E5): registrations and executions.
    invoke<AgentRegistration[]>("list_agents", { workspaceId })
      .then((r) => setAgents(r))
      .catch(fail);
    invoke<AgentExecution[]>("list_agent_executions", { workspaceId, limit: 50 })
      .then((r) => setAgentExecutions(r))
      .catch(fail);
    // E6 lists: skills, learning gaps, creator topics/drafts.
    invoke<Skill[]>("list_skills", { workspaceId, category: null })
      .then((r) => setSkills(r))
      .catch(fail);
    invoke<LearningGapProposal[]>("list_learning_gaps", { workspaceId, status: null })
      .then((r) => setLearningGaps(r))
      .catch(fail);
    // E6 第二批: retrospectives per project (default first project).
    if (projects.length > 0) {
      const pid = retroProject || projects[0].id;
      invoke<ProjectRetrospective[]>("list_retrospectives", { workspaceId, projectId: pid })
        .then((r) => setRetrospectives(r))
        .catch(fail);
    } else {
      setRetrospectives([]);
    }
    invoke<CreatorTopic[]>("list_creator_topics", { workspaceId })
      .then((r) => setCreatorTopics(r))
      .catch(fail);
    invoke<CreatorDraft[]>("list_creator_drafts", { workspaceId })
      .then((r) => setCreatorDrafts(r))
      .catch(fail);
  }, [workspaceId, retroProject]);

  useEffect(() => {
    reloadWorkspaces();
  }, [reloadWorkspaces]);

  useEffect(() => {
    reloadAll();
  }, [reloadAll]);

  // ---- Actions ----
  const run = async (fn: () => Promise<unknown>, after?: () => void) => {
    setBusy(true);
    setNotice(null);
    try {
      await fn();
      after?.();
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  const createWorkspace = () =>
    run(
      () => invoke<Workspace>("create_workspace", { name: workspaceName }),
      () => {
        setWorkspaceName("");
        reloadWorkspaces();
      },
    );

  const createArea = () =>
    run(
      () =>
        invoke<Area>("create_area", {
          workspaceId,
          name: areaName,
          description: null,
        }),
      () => {
        setAreaName("");
        reloadAll();
      },
    );

  const createGoal = () =>
    run(
      () =>
        invoke<Goal>("create_goal", {
          workspaceId,
          areaId: goalArea || null,
          name: goalName,
          description: null,
        }),
      () => {
        setGoalName("");
        reloadAll();
      },
    );

  const createProject = () =>
    run(
      () => invoke<Project>("create_project", { workspaceId, name: projectName }),
      () => {
        setProjectName("");
        reloadAll();
      },
    );

  const linkProject = (p: Project) =>
    run(
      () =>
        invoke<Project>("update_project", {
          workspaceId,
          projectId: p.id,
          name: p.name,
          description: p.description,
          status: p.status,
          goalId: projectGoal || null,
          areaId: projectArea || null,
        }),
      reloadAll,
    );

  const createTask = () =>
    run(
      () =>
        invoke<Task>("create_task", {
          workspaceId,
          projectId: taskProject || null,
          title: taskTitle,
          notes: null,
        }),
      () => {
        setTaskTitle("");
        reloadAll();
      },
    );

  const transitionTask = (t: Task, to: Task["status"]) =>
    run(
      () =>
        invoke<Task>("transition_task", {
          workspaceId,
          taskId: t.id,
          to,
        }),
      reloadAll,
    );

  const deleteEntity = (kind: "area" | "goal" | "project" | "task", id: string) =>
    run(
      () =>
        invoke<void>(
          kind === "area"
            ? "delete_area"
            : kind === "goal"
              ? "delete_goal"
              : kind === "project"
                ? "delete_project"
                : "delete_task",
          kind === "task" ? { workspaceId, taskId: id } : { workspaceId, [`${kind}Id`]: id },
        ),
      reloadAll,
    );

  const addAttachment = (taskId: string, file: File) =>
    run(async () => {
      const bytes = new Uint8Array(await file.arrayBuffer());
      await invoke<Attachment>("add_attachment", {
        workspaceId,
        ownerType: "task",
        ownerId: taskId,
        fileName: file.name,
        bytes: Array.from(bytes),
      });
    }, reloadAll);

  const removeAttachment = (attachmentId: string) =>
    run(() => invoke<void>("remove_attachment", { workspaceId, attachmentId }), reloadAll);

  const openAttachment = (a: Attachment) =>
    run(async () => {
      // WebView2 blocks programmatic blob downloads; the Rust side exports
      // the bytes to a temp file and hands it to the OS default app.
      const path = await invoke<string>("open_attachment_externally", {
        workspaceId,
        attachmentId: a.id,
      });
      setNotice(`已在系统默认程序中打开：${a.file_name}（临时副本：${path}）`);
    });

  const runSearch = () =>
    run(async () => {
      const result = await invoke<SearchHit[]>("search", {
        workspaceId,
        query,
        limit: 20,
      });
      setHits(result);
    });

  // ---- Learning & Knowledge actions (E4) ----
  const createRoadmap = () =>
    run(
      () =>
        invoke<Roadmap>("create_roadmap", {
          workspaceId,
          name: roadmapName,
          description: null,
        }),
      () => {
        setRoadmapName("");
        reloadAll();
      },
    );

  const createPlan = (roadmapId: string) =>
    run(
      () =>
        invoke<Plan>("create_plan", {
          workspaceId,
          roadmapId,
          name: planName,
          description: null,
        }),
      () => {
        setPlanName("");
        reloadAll();
      },
    );

  const cyclePlanStatus = (p: Plan) =>
    run(
      () =>
        invoke<Plan>("update_plan_status", {
          workspaceId,
          planId: p.id,
          status:
            p.status === "draft"
              ? "active"
              : p.status === "active"
                ? "completed"
                : "archived",
        }),
      reloadAll,
    );

  const createItem = (planId: string) =>
    run(
      () =>
        invoke<LearningItem>("create_item", {
          workspaceId,
          planId,
          kind: itemKind,
          title: itemTitle,
          expectedOutcome: null,
          acceptanceCriteria: null,
        }),
      () => {
        setItemTitle("");
        reloadAll();
      },
    );

  const linkItemProject = (item: LearningItem, projectId: string) =>
    run(
      () =>
        invoke<void>("link_item_to_project", {
          workspaceId,
          itemId: item.id,
          projectId,
        }),
      reloadAll,
    );

  const createTaskForItem = (item: LearningItem) =>
    run(
      () =>
        invoke<Task>("create_task_for_item", {
          workspaceId,
          itemId: item.id,
          title: `学习动作：${item.title}`,
        }),
      () => setNotice(`已通过 Work Management 创建任务（学习项：${item.title}）`),
    );

  const appendSession = (itemId: string) =>
    run(
      () =>
        invoke<LearningSession>("append_session", {
          workspaceId,
          itemId,
          method: "实践",
          durationMinutes: null,
          summary: sessionSummary || null,
          difficulty: null,
          takeaway: null,
          nextStep: null,
        }),
      () => {
        setSessionSummary("");
        reloadAll();
      },
    );

  const createKnowledgeRef = () =>
    run(
      () =>
        invoke<KnowledgeRef>("create_knowledge_ref", {
          workspaceId,
          title: knowledgeTitle,
          source: knowledgeSource,
          externalUri: knowledgeUri || null,
          locator: null,
          versionLabel: null,
        }),
      () => {
        setKnowledgeTitle("");
        setKnowledgeUri("");
        reloadAll();
      },
    );

  const cycleKnowledgeAvailability = (k: KnowledgeRef) =>
    run(
      () =>
        invoke<KnowledgeRef>("set_knowledge_availability", {
          workspaceId,
          knowledgeRefId: k.id,
          availability:
            k.availability === "available"
              ? "stale"
              : k.availability === "stale"
                ? "unavailable"
                : "available",
        }),
      reloadAll,
    );

  const deleteKnowledgeRef = (id: string) =>
    run(() => invoke<void>("delete_knowledge_ref", { workspaceId, knowledgeRefId: id }), reloadAll);

  // ---- Agent governance actions (E5) ----
  // 注册后需为 agent 主体创建 grant（授权不来自注册声明或审批，ADR-029）。
  const registerAgent = () =>
    run(async () => {
      const registration = await invoke<AgentRegistration>("register_agent", {
        workspaceId,
        displayName: agentName.trim(),
        adapterKind: "fake",
      });
      // Seed a default grant for the low-risk task capability so the slice
      // is usable end to end; admin can revoke it via the grant list.
      await invoke<string>("create_agent_grant", {
        workspaceId,
        subjectId: `agent:${registration.id}`,
        capabilities: ["work_management.task.create"],
        ttlMs: null,
      });
      setNotice(
        `已注册 agent「${registration.display_name}」并授予 work_management.task.create（可撤销）`,
      );
      setAgentName("");
    }, reloadAll);

  const updateAgentStatus = (id: string, newStatus: AgentRegistration["status"]) =>
    run(
      () =>
        invoke<AgentRegistration>("update_agent_registration", {
          workspaceId,
          registrationId: id,
          newStatus,
        }),
      reloadAll,
    );

  const proposeInvocation = () => {
    const registration = agents.find((a) => a.id === propAgent) ?? agents[0];
    if (!registration) {
      setNotice("请先注册一个 agent。");
      return;
    }
    run(async () => {
      const outcome = await invoke<AgentProposalOutcome>("propose_agent_invocation", {
        workspaceId,
        registrationId: registration.id,
        capabilityId: propCapability,
        contractVersion: "1.0.0",
        parametersJson: "{}",
      });
      if (outcome.status === "awaiting_approval" && outcome.display) {
        setPendingApproval({
          executionId: outcome.display.execution_id,
          display: outcome.display,
        });
        setNotice("高风险能力：请在审批面板确认后执行。");
      } else {
        setNotice(
          `调用完成：${outcome.execution.status}（操作 ${outcome.operation?.status ?? "n/a"}${outcome.summary ? `，${outcome.summary}` : ""}）`,
        );
      }
    }, reloadAll);
  };

  const decideApproval = (approve: boolean) => {
    if (!pendingApproval) return;
    const { executionId } = pendingApproval;
    setPendingApproval(null);
    run(async () => {
      const outcome = await invoke<AgentRunOutcome | null>("decide_approval", {
        workspaceId,
        executionId,
        approve,
      });
      setNotice(
        approve
          ? outcome
            ? `审批后执行：${outcome.status}（${outcome.summary}）`
            : "审批后无运行结果。"
          : "已拒绝并取消该执行。",
      );
    }, reloadAll);
  };

  const reconcileExecution = (id: string, complete: boolean) =>
    run(
      () =>
        invoke<AgentExecution>("reconcile_agent_execution", {
          workspaceId,
          executionId: id,
          complete,
        }),
      reloadAll,
    );

  // ---- E6: skills / learning gaps / creator actions ----
  const createSkill = () =>
    run(
      () =>
        invoke<Skill>("create_skill", {
          workspaceId,
          name: skillName.trim(),
          category: skillCategory.trim() || "general",
          notes: null,
        }),
      () => {
        setSkillName("");
        reloadAll();
      },
    );

  const proposeGap = () =>
    run(() => {
      const projectId = gapProject || projects[0]?.id;
      if (!projectId) {
        setNotice("请先创建一个项目，再从项目提出学习缺口。");
        return Promise.resolve();
      }
      return invoke<LearningGapProposal>("propose_learning_gap", {
        workspaceId,
        projectId,
        gap: gapText.trim(),
      });
    }, () => {
      setGapText("");
      reloadAll();
    });

  const confirmGap = (gap: LearningGapProposal) =>
    run(() => {
      if (!gapItem) {
        setNotice("请选择一个学习条目来承接该缺口。");
        return Promise.resolve();
      }
      return invoke<LearningGapProposal>("confirm_learning_gap", {
        workspaceId,
        gapId: gap.id,
        itemId: gapItem,
      });
    }, reloadAll);

  const dismissGap = (gap: LearningGapProposal) =>
    run(
      () =>
        invoke<LearningGapProposal>("dismiss_learning_gap", {
          workspaceId,
          gapId: gap.id,
        }),
      reloadAll,
    );

  const createTopic = () =>
    run(() => {
      return invoke<CreatorTopic>("create_creator_topic", {
        workspaceId,
        title: topicTitle.trim(),
        notes: null,
      });
    }, () => {
      setTopicTitle("");
      reloadAll();
    });

  const cycleTopicStatus = (t: CreatorTopic) =>
    run(
      () =>
        invoke<CreatorTopic>("update_creator_topic", {
          topic: {
            ...t,
            status:
              t.status === "Idea"
                ? "Selected"
                : t.status === "Selected"
                  ? "Parked"
                  : "Idea",
          },
        }),
      reloadAll,
    );

  const createDraft = (t: CreatorTopic) =>
    run(() => {
      return invoke<CreatorDraft>("create_creator_draft", {
        workspaceId,
        topicId: t.id,
        title: draftTitle.trim() || t.title,
      });
    }, () => {
      setDraftTitle("");
      reloadAll();
    });

  const appendVersion = (d: CreatorDraft) =>
    run(() => {
      if (!draftBody.trim()) {
        setNotice("版本正文为空。");
        return Promise.resolve();
      }
      return invoke<CreatorDraft>("append_creator_draft_version", {
        workspaceId,
        draftId: d.id,
        body: draftBody,
      });
    }, () => {
      setDraftBody("");
      reloadAll();
    });

  const transitionDraft = (d: CreatorDraft, newStatus: CreatorDraft["status"]) =>
    run(
      () =>
        invoke<CreatorDraft>("transition_creator_draft", {
          workspaceId,
          draftId: d.id,
          newStatus,
        }),
      reloadAll,
    );

  const submitReview = (d: CreatorDraft, decision: "Approved" | "Rejected") =>
    run(
      () =>
        invoke<CreatorDraftReview>("submit_creator_review", {
          workspaceId,
          draftId: d.id,
          versionNo: d.current_version_no,
          decision,
          comment: null,
        }),
      reloadAll,
    );

  // ---- E6 第二批: publish records + project retrospectives ----
  const recordPublish = (d: CreatorDraft, channel: "wechat" | "manual") =>
    run(async () => {
      const draft = await invoke<CreatorDraft>("record_publish", {
        workspaceId,
        draftId: d.id,
        channel,
        locator: null,
        notes: null,
      });
      setNotice(`已记录发布：${draft.title} v${draft.current_version_no}（${channel}，手动录入）`);
    }, reloadAll);

  const recordRetrospective = () =>
    run(async () => {
      const projectId = retroProject || projects[0]?.id;
      if (!projectId) {
        setNotice("请先创建一个项目。");
        return;
      }
      const sessions = retroSession ? [retroSession] : [];
      const retro = await invoke<ProjectRetrospective>("record_retrospective", {
        workspaceId,
        projectId,
        summary: retroSummary.trim(),
        sessionIds: sessions,
      });
      setNotice(
        `复盘已记录${retro.learning_session_refs.length > 0 ? `（回流学习记录 ${retro.learning_session_refs.length} 条）` : ""}`,
      );
      setRetroSummary("");
      setRetroSession("");
    }, reloadAll);

  // ---- Operation queue actions (E3-04) ----
  const cancelOperation = (op: OperationRow) =>
    run(async () => {
      await invoke<OperationRow>("request_operation_cancellation", {
        workspaceId,
        operationId: op.id,
      });
      setNotice(`已记录取消请求：${op.kind}（取消是否生效取决于能力声明与状态机）`);
      reloadAll();
    });

  const resumeOperation = (op: OperationRow) =>
    run(async () => {
      await invoke<OperationRow>("resume_operation", {
        workspaceId,
        operationId: op.id,
        note: "用户确认：结果已核对，可安全重试",
      });
      setNotice(`已恢复运行：${op.kind}`);
      reloadAll();
    });

  const redriveMessage = (m: QuarantinedMessage) =>
    run(async () => {
      const id = await invoke<string>("redrive_message", {
        workspaceId,
        messageId: m.id,
      });
      setNotice(`已重驱隔离事件 ${id}（保留事件 ID，操作者已记录）`);
      reloadAll();
    });

  const exportOperations = () =>
    run(async () => {
      const path = await invoke<string>("export_operations", { workspaceId });
      setNotice(`操作历史已导出：${path}`);
    });

  const purgeExpired = () =>
    run(async () => {
      const [operations, attempts, audit] = await invoke<[number, number, number]>(
        "purge_expired_data",
      );
      setNotice(`已清除过期数据：操作 ${operations}（含尝试 ${attempts}）、审计 ${audit}`);
      reloadAll();
    });

  // ---- Render ----
  return (
    <main>
      <h1>自磨工坊 · Personal AI Workbench</h1>
      <p>E2 纵向切片：Goal / Area / Project / Task、真实数据 Dashboard、范围化搜索与任务附件。</p>
      {notice && (
        <p role="alert">
          {notice}{" "}
          <button onClick={() => setNotice(null)}>关闭</button>
        </p>
      )}

      {/* Workspace picker + creation */}
      <section>
        <h2>工作空间</h2>
        {workspaces.length > 0 ? (
          <select
            value={workspaceId ?? ""}
            onChange={(e) => setWorkspaceId(e.target.value || null)}
            disabled={busy}
          >
            {workspaces.map((w) => (
              <option key={w.id} value={w.id}>
                {w.name}
              </option>
            ))}
          </select>
        ) : (
          <p>还没有工作空间——创建第一个：</p>
        )}
        {!workspaces.some((w) => w.id === workspaceId) || workspaces.length === 0 ? (
          <div>
            <input
              value={workspaceName}
              onChange={(e) => setWorkspaceName(e.target.value)}
              placeholder="工作空间名称（1–120 字符）"
              disabled={busy}
            />
            <button onClick={createWorkspace} disabled={busy || !workspaceName.trim()}>
              创建工作空间
            </button>
          </div>
        ) : null}
      </section>

      {workspaceId && (
        <>
          {/* Real-data dashboard */}
          <section>
            <h2>仪表板（真实数据）</h2>
            {dashboard ? (
              <ul>
                <li>领域：{dashboard.area_count}</li>
                <li>目标：{dashboard.goal_count}</li>
                <li>
                  项目：
                  {dashboard.projects_by_status.map(([s, n]) => `${s}=${n}`).join("，")}
                </li>
                <li>
                  任务：
                  {dashboard.tasks_by_status.map(([s, n]) => `${s}=${n}`).join("，")}
                </li>
                <li>附件：{dashboard.attachment_count}</li>
                <li>
                  学习：路线 {dashboard.roadmap_count} · 计划 {dashboard.learning_plan_count} ·
                  项 {dashboard.learning_item_count} · Session {dashboard.learning_session_count}
                </li>
              </ul>
            ) : (
              <p>加载中…</p>
            )}
          </section>

          {/* Scoped search */}
          <section>
            <h2>搜索</h2>
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="搜索项目与任务（当前工作空间内）"
              disabled={busy}
            />
            <button onClick={runSearch} disabled={busy || !query.trim()}>
              搜索
            </button>
            {hits && (
              <ul>
                {hits.length === 0 && <li>无结果。</li>}
                {hits.map((h) => (
                  <li key={`${h.kind}-${h.id}`}>
                    [{h.kind === "project" ? "项目" : "任务"}] {h.title}
                  </li>
                ))}
              </ul>
            )}
          </section>

          {/* Areas */}
          <section>
            <h2>领域</h2>
            <input
              value={areaName}
              onChange={(e) => setAreaName(e.target.value)}
              placeholder="领域名称（1–120 字符）"
              disabled={busy}
            />
            <button onClick={createArea} disabled={busy || !areaName.trim()}>
              创建领域
            </button>
            <ul>
              {areas.map((a) => (
                <li key={a.id}>
                  {a.name}{" "}
                  <button onClick={() => deleteEntity("area", a.id)} disabled={busy}>
                    删除
                  </button>
                </li>
              ))}
            </ul>
          </section>

          {/* Goals */}
          <section>
            <h2>目标</h2>
            <input
              value={goalName}
              onChange={(e) => setGoalName(e.target.value)}
              placeholder="目标名称（1–200 字符）"
              disabled={busy}
            />
            <select
              value={goalArea}
              onChange={(e) => setGoalArea(e.target.value)}
              disabled={busy}
            >
              <option value="">（不归属领域）</option>
              {areas.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.name}
                </option>
              ))}
            </select>
            <button onClick={createGoal} disabled={busy || !goalName.trim()}>
              创建目标
            </button>
            <ul>
              {goals.map((g) => (
                <li key={g.id}>
                  {g.name}
                  {g.area_id && `（${areas.find((a) => a.id === g.area_id)?.name ?? "?"}）`}{" "}
                  <button onClick={() => deleteEntity("goal", g.id)} disabled={busy}>
                    删除
                  </button>
                </li>
              ))}
            </ul>
          </section>

          {/* Projects */}
          <section>
            <h2>项目</h2>
            <input
              value={projectName}
              onChange={(e) => setProjectName(e.target.value)}
              placeholder="项目名称（1–200 字符）"
              disabled={busy}
            />
            <button onClick={createProject} disabled={busy || !projectName.trim()}>
              创建项目
            </button>
            <ul>
              {projects.map((p) => (
                <li key={p.id}>
                  <strong>{p.name}</strong>（{p.status}）{" "}
                  <button onClick={() => deleteEntity("project", p.id)} disabled={busy}>
                    删除
                  </button>
                  <div>
                    关联目标：
                    <select
                      value={p.goal_id ?? ""}
                      onChange={(e) => {
                        setProjectGoal(e.target.value);
                        setProjectArea(p.area_id ?? "");
                        linkProject({ ...p, goal_id: e.target.value || null });
                      }}
                      disabled={busy}
                    >
                      <option value="">（无）</option>
                      {goals.map((g) => (
                        <option key={g.id} value={g.id}>
                          {g.name}
                        </option>
                      ))}
                    </select>
                    关联领域：
                    <select
                      value={p.area_id ?? ""}
                      onChange={(e) => {
                        setProjectArea(e.target.value);
                        setProjectGoal(p.goal_id ?? "");
                        linkProject({ ...p, area_id: e.target.value || null });
                      }}
                      disabled={busy}
                    >
                      <option value="">（无）</option>
                      {areas.map((a) => (
                        <option key={a.id} value={a.id}>
                          {a.name}
                        </option>
                      ))}
                    </select>
                  </div>
                </li>
              ))}
            </ul>
          </section>

          {/* Tasks + attachments */}
          <section>
            <h2>任务</h2>
            <input
              value={taskTitle}
              onChange={(e) => setTaskTitle(e.target.value)}
              placeholder="任务标题（1–200 字符）"
              disabled={busy}
            />
            <select
              value={taskProject}
              onChange={(e) => setTaskProject(e.target.value)}
              disabled={busy}
            >
              <option value="">（独立任务）</option>
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
            <button onClick={createTask} disabled={busy || !taskTitle.trim()}>
              创建任务
            </button>
            <ul>
              {tasks.map((t) => (
                <li key={t.id}>
                  <strong>{t.title}</strong>（{t.status}）{" "}
                  {t.status === "todo" && (
                    <button
                      onClick={() => transitionTask(t, "in_progress")}
                      disabled={busy}
                    >
                      开始
                    </button>
                  )}
                  {t.status === "in_progress" && (
                    <>
                      <button onClick={() => transitionTask(t, "done")} disabled={busy}>
                        完成
                      </button>
                      <button onClick={() => transitionTask(t, "todo")} disabled={busy}>
                        退回
                      </button>
                    </>
                  )}
                  {(t.status === "todo" || t.status === "in_progress") && (
                    <button onClick={() => transitionTask(t, "cancelled")} disabled={busy}>
                      取消
                    </button>
                  )}
                  <button onClick={() => deleteEntity("task", t.id)} disabled={busy}>
                    删除
                  </button>
                  <div>
                    <label>
                      添加附件：
                      <input
                        type="file"
                        disabled={busy}
                        onChange={(e) => {
                          const file = e.target.files?.[0];
                          if (file) addAttachment(t.id, file);
                          e.target.value = "";
                        }}
                      />
                    </label>
                    <ul>
                      {(taskAttachments[t.id] ?? []).map((a) => (
                        <li key={a.id}>
                          {a.file_name}（{a.size_bytes} 字节）{" "}
                          <button onClick={() => openAttachment(a)} disabled={busy}>
                            打开
                          </button>{" "}
                          <button onClick={() => removeAttachment(a.id)} disabled={busy}>
                            移除
                          </button>
                        </li>
                      ))}
                    </ul>
                  </div>
                </li>
              ))}
            </ul>
          </section>

          {/* Operation queue (E3-04) */}
          <section>
            <h2>操作队列</h2>
            <p>
              持久操作与隔离事件（当前工作空间）。导出先于清除；清除为显式动作，无后台自动清除。
            </p>
            <button onClick={exportOperations} disabled={busy}>
              导出操作历史（JSON）
            </button>{" "}
            <button onClick={purgeExpired} disabled={busy}>
              清除过期数据（默认策略）
            </button>
            <ul>
              {operations.length === 0 && <li>无操作记录。</li>}
              {operations.map((op) => (
                <li key={op.id}>
                  <strong>{op.kind}</strong>（{op.status}）
                  {op.next_attempt_at !== null &&
                    ` · 下次尝试 ${new Date(op.next_attempt_at).toLocaleString()}`}
                  {op.cancellation_requested_at !== null && " · 已请求取消"}
                  {op.status === "running" && (
                    <button onClick={() => cancelOperation(op)} disabled={busy}>
                      请求取消
                    </button>
                  )}
                  {op.status === "outcome_unknown" && (
                    <button onClick={() => resumeOperation(op)} disabled={busy}>
                      核对后恢复
                    </button>
                  )}
                </li>
              ))}
            </ul>
            <h3>隔离事件</h3>
            <ul>
              {quarantined.length === 0 && <li>无隔离事件。</li>}
              {quarantined.map((m) => (
                <li key={m.id}>
                  {m.event_type}（尝试 {m.attempts} 次，重驱 {m.redrive_count} 次
                  {m.last_error_category && `，原因：${m.last_error_category}`}
                  {m.last_redrive_by && `，上次操作者：${m.last_redrive_by}`}）{" "}
                  <button onClick={() => redriveMessage(m)} disabled={busy}>
                    重驱
                  </button>
                </li>
              ))}
            </ul>
          </section>

          {/* Learning & Growth (E4) */}
          <section>
            <h2>学习</h2>
            <input
              value={roadmapName}
              onChange={(e) => setRoadmapName(e.target.value)}
              placeholder="路线名称（1–200 字符）"
              disabled={busy}
            />
            <button onClick={createRoadmap} disabled={busy || !roadmapName.trim()}>
              创建路线
            </button>
            <ul>
              {roadmaps.map((r) => (
                <li key={r.id}>
                  <strong>{r.name}</strong>
                  <div>
                    计划：
                    <input
                      value={planName}
                      onChange={(e) => setPlanName(e.target.value)}
                      placeholder="计划名称"
                      disabled={busy}
                    />
                    <button
                      onClick={() => createPlan(r.id)}
                      disabled={busy || !planName.trim()}
                    >
                      创建计划
                    </button>
                    <ul>
                      {plans
                        .filter((p) => p.roadmap_id === r.id)
                        .map((p) => (
                          <li key={p.id}>
                            {p.name}（{p.status}）{" "}
                            <button onClick={() => cyclePlanStatus(p)} disabled={busy}>
                              推进状态
                            </button>{" "}
                            <div>
                              项：
                              <input
                                value={itemTitle}
                                onChange={(e) => setItemTitle(e.target.value)}
                                placeholder="学习项标题"
                                disabled={busy}
                              />
                              <select
                                value={itemKind}
                                onChange={(e) =>
                                  setItemKind(e.target.value as LearningItem["kind"])
                                }
                                disabled={busy}
                              >
                                <option value="concept">概念</option>
                                <option value="reading">阅读</option>
                                <option value="exercise">练习</option>
                                <option value="experiment">实验</option>
                                <option value="implementation">实现</option>
                                <option value="review">复盘</option>
                              </select>
                              <button
                                onClick={() => createItem(p.id)}
                                disabled={busy || !itemTitle.trim()}
                              >
                                添加学习项
                              </button>
                              <ul>
                                {learningItems
                                  .filter((i) => i.plan_id === p.id)
                                  .map((i) => (
                                    <li key={i.id}>
                                      {i.title}（{i.kind}，{i.status}）{" "}
                                      <button
                                        onClick={() =>
                                          invoke<LearningItem>("transition_item", {
                                            workspaceId,
                                            itemId: i.id,
                                            to:
                                              i.status === "todo"
                                                ? "in_progress"
                                                : i.status === "in_progress"
                                                  ? "done"
                                                  : "todo",
                                          })
                                            .then(() => reloadAll())
                                            .catch(fail)
                                        }
                                        disabled={busy || i.status === "done" || i.status === "cancelled"}
                                      >
                                        推进状态
                                      </button>{" "}
                                      <select
                                        value=""
                                        onChange={(e) => {
                                          if (e.target.value)
                                            linkItemProject(i, e.target.value);
                                        }}
                                        disabled={busy}
                                      >
                                        <option value="">关联项目…</option>
                                        {projects.map((pr) => (
                                          <option key={pr.id} value={pr.id}>
                                            {pr.name}
                                          </option>
                                        ))}
                                      </select>{" "}
                                      <button
                                        onClick={() => createTaskForItem(i)}
                                        disabled={busy}
                                      >
                                        生成任务
                                      </button>
                                      <div>
                                        Session：
                                        <input
                                          value={sessionItem === i.id ? sessionSummary : ""}
                                          onChange={(e) => {
                                            setSessionItem(i.id);
                                            setSessionSummary(e.target.value);
                                          }}
                                          placeholder="学习摘要"
                                          disabled={busy}
                                        />
                                        <button
                                          onClick={() => appendSession(i.id)}
                                          disabled={
                                            busy || sessionItem !== i.id || !sessionSummary.trim()
                                          }
                                        >
                                          记录 Session
                                        </button>
                                        <ul>
                                          {learningSessions
                                            .filter((s) => s.item_id === i.id)
                                            .map((s) => (
                                              <li key={s.id}>
                                                {new Date(s.occurred_at).toLocaleString()}：
                                                {s.summary ?? "(无摘要)"}
                                                {s.duration_minutes !== null &&
                                                  `（${s.duration_minutes} 分钟）`}
                                              </li>
                                            ))}
                                        </ul>
                                      </div>
                                    </li>
                                  ))}
                              </ul>
                            </div>
                          </li>
                        ))}
                    </ul>
                  </div>
                </li>
              ))}
            </ul>
          </section>

          {/* Knowledge & Context (E4) */}
          <section>
            <h2>知识引用</h2>
            <input
              value={knowledgeTitle}
              onChange={(e) => setKnowledgeTitle(e.target.value)}
              placeholder="知识标题（1–200 字符）"
              disabled={busy}
            />
            <select
              value={knowledgeSource}
              onChange={(e) => setKnowledgeSource(e.target.value as KnowledgeRef["source"])}
              disabled={busy}
            >
              <option value="web">Web</option>
              <option value="lingshu">Lingshu</option>
              <option value="hindsight">Hindsight</option>
              <option value="local">本地</option>
            </select>
            <input
              value={knowledgeUri}
              onChange={(e) => setKnowledgeUri(e.target.value)}
              placeholder="外部 URI（可选）"
              disabled={busy}
            />
            <button onClick={createKnowledgeRef} disabled={busy || !knowledgeTitle.trim()}>
              添加引用
            </button>
            <ul>
              {knowledgeRefs.map((k) => (
                <li key={k.id}>
                  {k.title}（{k.source}，{k.availability}）{" "}
                  <button onClick={() => cycleKnowledgeAvailability(k)} disabled={busy}>
                    切换可用性
                  </button>{" "}
                  <button onClick={() => deleteKnowledgeRef(k.id)} disabled={busy}>
                    删除
                  </button>
                </li>
              ))}
            </ul>
          </section>

          {/* Agent governance (E5) */}
          <section>
            <h2>Agent 协作</h2>
            <p>
              注册的 agent 以自身主体身份经能力门禁调用（默认拒绝，按 grant 授权）；高风险能力需人工审批，
              审批绑定精确计划摘要；不确定外部结果保持未知状态等待对账，不盲重试。
            </p>
            <input
              value={agentName}
              onChange={(e) => setAgentName(e.target.value)}
              placeholder="agent 名称"
              disabled={busy}
            />
            <button onClick={registerAgent} disabled={busy || !agentName.trim()}>
              注册 agent
            </button>
            <ul>
              {agents.length === 0 && <li>无注册 agent。</li>}
              {agents.map((a) => (
                <li key={a.id}>
                  <strong>{a.display_name}</strong>（{a.adapter_kind}，{a.status}）{" "}
                  {a.status === "Active" && (
                    <button onClick={() => updateAgentStatus(a.id, "Suspended")} disabled={busy}>
                      暂停
                    </button>
                  )}
                  {a.status === "Suspended" && (
                    <button onClick={() => updateAgentStatus(a.id, "Active")} disabled={busy}>
                      恢复
                    </button>
                  )}
                  {a.status !== "Retired" && (
                    <button onClick={() => updateAgentStatus(a.id, "Retired")} disabled={busy}>
                      退役
                    </button>
                  )}
                </li>
              ))}
            </ul>
            <h3>发起受控调用</h3>
            <select
              value={propAgent || agents[0]?.id || ""}
              onChange={(e) => setPropAgent(e.target.value)}
              disabled={busy || agents.length === 0}
            >
              {agents.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.display_name}
                </option>
              ))}
            </select>
            <select
              value={propCapability}
              onChange={(e) => setPropCapability(e.target.value)}
              disabled={busy}
            >
              <option value="work_management.task.create">
                work_management.task.create（低风险，需 grant）
              </option>
              <option value="data.purge">data.purge（高风险，需审批 + grant）</option>
            </select>
            <button onClick={proposeInvocation} disabled={busy || agents.length === 0}>
              发起调用
            </button>
            {pendingApproval && (
              <div>
                <h4>待审批（绑定精确计划）</h4>
                <ul>
                  <li>能力：{pendingApproval.display.capability_id}</li>
                  <li>契约版本：{pendingApproval.display.capability_contract_version}</li>
                  <li>目标：{pendingApproval.display.target_identity}</li>
                  <li>风险：{pendingApproval.display.risk_class}</li>
                  <li>
                    有效期至：
                    {new Date(pendingApproval.display.expires_at_ms).toLocaleString()}
                  </li>
                  <li>计划摘要：{pendingApproval.display.side_effect_set.join("、")}</li>
                </ul>
                <button onClick={() => decideApproval(true)} disabled={busy}>
                  批准执行
                </button>{" "}
                <button onClick={() => decideApproval(false)} disabled={busy}>
                  拒绝
                </button>
              </div>
            )}
            <h3>执行记录</h3>
            <ul>
              {agentExecutions.length === 0 && <li>无执行记录。</li>}
              {agentExecutions.map((e) => (
                <li key={e.id}>
                  <strong>{e.capability_id}</strong>（{e.status}，契约 {e.capability_contract_version}）
                  {e.operation_id !== null && ` · 操作 ${e.operation_id.slice(0, 8)}…`}
                  {(e.status === "OutcomeUnknown" || e.status === "TimedOut") && (
                    <>
                      {" "}
                      <button onClick={() => reconcileExecution(e.id, true)} disabled={busy}>
                        对账：已完成
                      </button>{" "}
                      <button onClick={() => reconcileExecution(e.id, false)} disabled={busy}>
                        对账：已失败
                      </button>
                    </>
                  )}
                </li>
              ))}
            </ul>
          </section>

          {/* Skill matrix & growth assessment (E6-01) */}
          <section>
            <h2>技能评估</h2>
            <p>
              技能等级只能由你基于证据确认（评估必须引用至少一条证据，保留尺度版本与评估历史）；
              学习完成度与时间不能被误读为能力证明。
            </p>
            <input
              value={skillName}
              onChange={(e) => setSkillName(e.target.value)}
              placeholder="技能名称"
              disabled={busy}
            />
            <input
              value={skillCategory}
              onChange={(e) => setSkillCategory(e.target.value)}
              placeholder="分类（可选）"
              disabled={busy}
            />
            <button onClick={createSkill} disabled={busy || !skillName.trim()}>
              添加技能
            </button>
            <ul>
              {skills.length === 0 && <li>暂无技能。添加技能后再录入证据与评估。</li>}
              {skills.map((s) => (
                <li key={s.id}>
                  <strong>{s.name}</strong>（{s.category}，尺度 v{s.scale_version}）
                </li>
              ))}
            </ul>
            <p className="muted">
              证据与评估通过命令桥录入（create_skill_evidence / create_assessment）；
              UI 逐步完善中。
            </p>
          </section>

          {/* Project learning gaps (E6-02) */}
          <section>
            <h2>学习缺口</h2>
            <p>
              项目复盘提出待确认的学习需求；确认后绑定学习条目；搁置保留决策历史。
            </p>
            <select
              value={gapProject || projects[0]?.id || ""}
              onChange={(e) => setGapProject(e.target.value)}
              disabled={busy || projects.length === 0}
            >
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
            <input
              value={gapText}
              onChange={(e) => setGapText(e.target.value)}
              placeholder="缺口描述（1–500 字符）"
              disabled={busy}
            />
            <button onClick={proposeGap} disabled={busy || !gapText.trim()}>
              提出缺口
            </button>
            <ul>
              {learningGaps.length === 0 && <li>暂无学习缺口提议。</li>}
              {learningGaps.map((g) => (
                <li key={g.id}>
                  <strong>{g.gap}</strong>（{g.status}
                  {g.confirmed_item_id !== null && " · 已绑定学习条目"}）{" "}
                  {g.status === "Proposed" && (
                    <>
                      <select
                        value={gapItem}
                        onChange={(e) => setGapItem(e.target.value)}
                        disabled={busy}
                      >
                        <option value="">选择学习条目…</option>
                        {learningItems.map((i) => (
                          <option key={i.id} value={i.id}>
                            {i.title}
                          </option>
                        ))}
                      </select>
                      <button
                        onClick={() => confirmGap(g)}
                        disabled={busy || !gapItem}
                      >
                        确认
                      </button>{" "}
                      <button onClick={() => dismissGap(g)} disabled={busy}>
                        搁置
                      </button>
                    </>
                  )}
                </li>
              ))}
            </ul>
          </section>

          {/* Creator content pipeline (E6-03) */}
          <section>
            <h2>内容创作</h2>
            <p>选题 → 草稿（版本不可变）→ 审核（绑定精确版本）。发布自动化待官方接口验证。</p>
            <input
              value={topicTitle}
              onChange={(e) => setTopicTitle(e.target.value)}
              placeholder="选题标题"
              disabled={busy}
            />
            <button onClick={createTopic} disabled={busy || !topicTitle.trim()}>
              添加选题
            </button>
            <ul>
              {creatorTopics.length === 0 && <li>暂无选题。</li>}
              {creatorTopics.map((t) => (
                <li key={t.id}>
                  <strong>{t.title}</strong>（{t.status}）{" "}
                  <button onClick={() => cycleTopicStatus(t)} disabled={busy}>
                    推进状态
                  </button>{" "}
                  <button onClick={() => createDraft(t)} disabled={busy}>
                    建草稿
                  </button>
                </li>
              ))}
            </ul>
            <input
              value={draftTitle}
              onChange={(e) => setDraftTitle(e.target.value)}
              placeholder="草稿标题（可选，默认同选题）"
              disabled={busy}
            />
            <ul>
              {creatorDrafts.length === 0 && <li>暂无草稿。</li>}
              {creatorDrafts.map((d) => (
                <li key={d.id}>
                  <strong>{d.title}</strong>
                  （v{d.current_version_no}，{d.status}）{" "}
                  <textarea
                    value={draftBody}
                    onChange={(e) => setDraftBody(e.target.value)}
                    placeholder="新版本正文"
                    disabled={busy}
                    rows={2}
                  />
                  <button
                    onClick={() => appendVersion(d)}
                    disabled={busy || !draftBody.trim()}
                  >
                    保存新版本
                  </button>{" "}
                  {d.status === "Writing" && (
                    <button
                      onClick={() => transitionDraft(d, "InReview")}
                      disabled={busy}
                    >
                      提交审核
                    </button>
                  )}
                  {d.status === "InReview" && (
                    <>
                      <button
                        onClick={() => submitReview(d, "Approved")}
                        disabled={busy}
                      >
                        通过 v{d.current_version_no}
                      </button>{" "}
                      <button
                        onClick={() => submitReview(d, "Rejected")}
                        disabled={busy}
                      >
                        驳回
                      </button>
                    </>
                  )}
                  {d.status === "InReview" && (
                    <button
                      onClick={() => transitionDraft(d, "Writing")}
                      disabled={busy}
                    >
                      退回写作
                    </button>
                  )}
                  {d.status === "Approved" && (
                    <>
                      <button
                        onClick={() => transitionDraft(d, "Writing")}
                        disabled={busy}
                      >
                        开启新版本
                      </button>{" "}
                      <button
                        onClick={() => recordPublish(d, "wechat")}
                        disabled={busy}
                      >
                        记录发布（公众号，手动录入）
                      </button>{" "}
                      <button
                        onClick={() => recordPublish(d, "manual")}
                        disabled={busy}
                      >
                        记录发布（其他渠道）
                      </button>
                    </>
                  )}
                  {d.status === "Published" && <span>已发布</span>}
                </li>
              ))}
            </ul>
          </section>

          {/* Project retrospective with learning backflow (E6 第二批) */}
          <section>
            <h2>项目复盘</h2>
            <p>
              学习产物回流项目复盘：复盘引用的学习记录必须属于已关联本项目的学习条目；
              记录为追加式，事件版本化；发布与复盘均为手动录入，不自动发布。
            </p>
            <select
              value={retroProject || projects[0]?.id || ""}
              onChange={(e) => setRetroProject(e.target.value)}
              disabled={busy || projects.length === 0}
            >
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
            <input
              value={retroSummary}
              onChange={(e) => setRetroSummary(e.target.value)}
              placeholder="复盘结论（1–2000 字符）"
              disabled={busy}
            />
            <select
              value={retroSession}
              onChange={(e) => setRetroSession(e.target.value)}
              disabled={busy}
            >
              <option value="">不引用学习记录</option>
              {learningSessions.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.summary || s.id.slice(0, 8)}
                </option>
              ))}
            </select>
            <button
              onClick={recordRetrospective}
              disabled={busy || !retroSummary.trim() || projects.length === 0}
            >
              记录复盘
            </button>
            <ul>
              {retrospectives.length === 0 && <li>暂无复盘记录。</li>}
              {retrospectives.map((r) => (
                <li key={r.id}>
                  <strong>{r.summary}</strong>
                  {r.learning_session_refs.length > 0 &&
                    ` · 回流学习记录 ${r.learning_session_refs.length} 条`}
                </li>
              ))}
            </ul>
          </section>
        </>
      )}
    </main>
  );
}

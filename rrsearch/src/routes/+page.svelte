<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  // ── Types ──────────────────────────────────────────────────────────────────
  interface Tab {
    id: string;
    title: string;
    url: string;
    workspaceId: string | null;
    favicon?: string;
  }

  interface Workspace {
    id: string;
    name: string;
    color: string;
    created_at: string;
  }

  interface Session {
    id: string;
    name: string;
    workspace_id: string | null;
    started_at: string;
    ended_at: string | null;
    decision_note: string | null;
  }

  interface Clip {
    id: string;
    session_id: string | null;
    url: string;
    page_title: string;
    text: string;
    clipped_at: string;
  }

  // ── State ──────────────────────────────────────────────────────────────────
  let tabs = $state<Tab[]>([
    { id: "1", title: "new tab", url: "", workspaceId: null }
  ]);
  let activeTabId = $state("1");
  let addressInput = $state("");
  let workspaces = $state<Workspace[]>([]);
  let activeWorkspaceId = $state<string | null>(null);
  let sessions = $state<Session[]>([]);
  let activeSessionId = $state<string | null>(null);
  let clips = $state<Clip[]>([]);

  // panels
  let showSidebar = $state(false);
  let sidebarTab = $state<"sessions" | "clips" | "workspaces">("sessions");
  let showTerminal = $state(false);
  let showNewWorkspace = $state(false);
  let showSessionPrompt = $state(false);
  let showEndSession = $state(false);
  let terminalInput = $state("");
  let terminalHistory = $state<string[]>(["rrsearch terminal v0.1.0", "type 'help' for commands", "─────────────────────────"]);
  let newWorkspaceName = $state("");
  let newWorkspaceColor = $state("#00ff41");
  let sessionPromptName = $state("");
  let sessionDecision = $state("");
  let tabCounter = $state(2);

  // drag
  let dragTabId = $state<string | null>(null);

  // ── Derived ────────────────────────────────────────────────────────────────
  const activeTab = $derived(tabs.find(t => t.id === activeTabId));
  const workspaceTabs = $derived(
    activeWorkspaceId
      ? tabs.filter(t => t.workspaceId === activeWorkspaceId)
      : tabs
  );

  // ── Lifecycle ─────────────────────────────────────────────────────────────
  onMount(async () => {
    workspaces = await invoke("list_workspaces");
    sessions = await invoke("list_sessions");
  });

  // ── Tab management ─────────────────────────────────────────────────────────
  function newTab() {
    const id = String(tabCounter++);
    tabs = [...tabs, { id, title: "new tab", url: "", workspaceId: activeWorkspaceId }];
    activeTabId = id;
    addressInput = "";
  }

  function closeTab(id: string) {
    if (tabs.length === 1) return;
    const idx = tabs.findIndex(t => t.id === id);
    tabs = tabs.filter(t => t.id !== id);
    if (activeTabId === id) {
      activeTabId = tabs[Math.max(0, idx - 1)].id;
    }
  }

  function switchTab(id: string) {
    activeTabId = id;
    const t = tabs.find(t => t.id === id);
    addressInput = t?.url ?? "";
  }

  // ── Navigation ─────────────────────────────────────────────────────────────
  async function navigate(event?: Event) {
    event?.preventDefault();
    if (!addressInput.trim()) return;
    const resolved: string = await invoke("resolve_url", { input: addressInput });
    tabs = tabs.map(t =>
      t.id === activeTabId
        ? { ...t, url: resolved, title: resolved.replace(/^https?:\/\//, "").slice(0, 30) }
        : t
    );
    addressInput = resolved;

    // auto-detect research session — 3+ navigations without one
    if (!activeSessionId && tabs.filter(t => t.url).length >= 3) {
      showSessionPrompt = true;
      sessionPromptName = "research " + new Date().toLocaleDateString();
    }

    // log to active session
    if (activeSessionId) {
      await invoke("log_site", {
        sessionId: activeSessionId,
        url: resolved,
        title: activeTab?.title ?? resolved,
      });
    }
  }

  function handleAddressKey(e: KeyboardEvent) {
    if (e.key === "Enter") navigate();
    if (e.key === "Escape") addressInput = activeTab?.url ?? "";
  }

  // ── Workspaces ─────────────────────────────────────────────────────────────
  async function createWorkspace() {
    if (!newWorkspaceName.trim()) return;
    const ws: Workspace = await invoke("create_workspace", {
      name: newWorkspaceName.trim(),
      color: newWorkspaceColor,
    });
    workspaces = [...workspaces, ws];
    activeWorkspaceId = ws.id;
    showNewWorkspace = false;
    newWorkspaceName = "";
  }

  // ── Sessions ───────────────────────────────────────────────────────────────
  async function startSession() {
    if (!sessionPromptName.trim()) return;
    const session: Session = await invoke("create_session", {
      name: sessionPromptName.trim(),
      workspaceId: activeWorkspaceId,
    });
    sessions = [session, ...sessions];
    activeSessionId = session.id;
    showSessionPrompt = false;
    sessionPromptName = "";
  }

  async function endSession() {
    if (!activeSessionId) return;
    await invoke("end_session", {
      sessionId: activeSessionId,
      decisionNote: sessionDecision || null,
    });
    sessions = await invoke("list_sessions");
    activeSessionId = null;
    sessionDecision = "";
    showEndSession = false;
  }

  // ── Clips ──────────────────────────────────────────────────────────────────
  async function saveClip(text: string) {
    if (!text.trim() || !activeTab) return;
    const clip: Clip = await invoke("save_clip", {
      sessionId: activeSessionId,
      url: activeTab.url,
      pageTitle: activeTab.title,
      text: text.trim(),
    });
    clips = [clip, ...clips];
  }

  async function loadClips() {
    clips = await invoke("get_clips", { sessionId: activeSessionId ?? null });
  }

  // ── Terminal ───────────────────────────────────────────────────────────────
  function runTerminalCmd() {
    const cmd = terminalInput.trim();
    if (!cmd) return;
    terminalHistory = [...terminalHistory, `> ${cmd}`];
    const out = evalCmd(cmd);
    if (out) terminalHistory = [...terminalHistory, out];
    terminalInput = "";
  }

  function evalCmd(cmd: string): string {
    const [c, ...args] = cmd.split(" ");
    switch (c.toLowerCase()) {
      case "help":
        return "commands: new, close, tabs, session, clip, clear, goto <url>";
      case "new":
        newTab();
        return "new tab opened";
      case "close":
        closeTab(activeTabId);
        return "tab closed";
      case "tabs":
        return tabs.map((t, i) => `[${i}] ${t.title || "new tab"}`).join("\n");
      case "session":
        showSessionPrompt = true;
        return "opening session dialog...";
      case "clip":
        return "select text on page to clip it";
      case "clear":
        terminalHistory = [];
        return "";
      case "goto":
        if (args[0]) { addressInput = args.join(" "); navigate(); return `navigating to ${args[0]}`; }
        return "usage: goto <url>";
      default:
        return `unknown command: ${c}`;
    }
  }

  function handleTerminalKey(e: KeyboardEvent) {
    if (e.key === "Enter") runTerminalCmd();
  }

  // ── Drag tabs ──────────────────────────────────────────────────────────────
  function onDragStart(id: string) { dragTabId = id; }
  function onDragOver(e: DragEvent) { e.preventDefault(); }
  function onDrop(targetId: string) {
    if (!dragTabId || dragTabId === targetId) return;
    const from = tabs.findIndex(t => t.id === dragTabId);
    const to = tabs.findIndex(t => t.id === targetId);
    const reordered = [...tabs];
    const [moved] = reordered.splice(from, 1);
    reordered.splice(to, 0, moved);
    tabs = reordered;
    dragTabId = null;
  }

  // ── Sidebar toggle ─────────────────────────────────────────────────────────
  function openSidebar(tab: typeof sidebarTab) {
    sidebarTab = tab;
    showSidebar = true;
    if (tab === "clips") loadClips();
  }

  function formatDate(iso: string) {
    return new Date(iso).toLocaleDateString("en-US", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
  }
</script>

<!-- ── Root Layout ─────────────────────────────────────────────────────────── -->
<div class="browser">

  <!-- ── Tab Bar ──────────────────────────────────────────────────────────── -->
  <div class="tabbar">
    <div class="tabs-scroll">
      {#each tabs as tab (tab.id)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="tab {tab.id === activeTabId ? 'tab--active' : ''}"
          draggable="true"
          ondragstart={() => onDragStart(tab.id)}
          ondragover={onDragOver}
          ondrop={() => onDrop(tab.id)}
          onclick={() => switchTab(tab.id)}
          role="tab"
          tabindex="0"
          onkeydown={(e) => e.key === 'Enter' && switchTab(tab.id)}
        >
          <span class="tab-title">{tab.title || "new tab"}</span>
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <span class="tab-close" onclick={(e) => { e.stopPropagation(); closeTab(tab.id); }} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && closeTab(tab.id)}>×</span>
        </div>
      {/each}
    </div>
    <button class="tab-new" onclick={newTab} title="new tab">+</button>
    <div class="tabbar-actions">
      <button class="icon-btn" onclick={() => openSidebar("sessions")} title="sessions">◎</button>
      <button class="icon-btn" onclick={() => openSidebar("clips")} title="clips">⌗</button>
      <button class="icon-btn" onclick={() => openSidebar("workspaces")} title="workspaces">▦</button>
      <button class="icon-btn" onclick={() => (showTerminal = !showTerminal)} title="terminal">&gt;_</button>
    </div>
  </div>

  <!-- ── Address Bar ──────────────────────────────────────────────────────── -->
  <div class="addressbar">
    {#if activeSessionId}
      <div class="session-badge">
        <span class="session-dot">●</span>
        <span>{sessions.find(s => s.id === activeSessionId)?.name ?? "session"}</span>
        <button class="session-end-btn" onclick={() => (showEndSession = true)}>end</button>
      </div>
    {/if}
    <form class="address-form" onsubmit={navigate}>
      <span class="prompt">rrsearch://</span>
      <input
        class="address-input"
        bind:value={addressInput}
        onkeydown={handleAddressKey}
        placeholder="domain or search..."
        spellcheck="false"
        autocomplete="off"
      />
      <button type="submit" class="go-btn">→</button>
    </form>
    {#if activeWorkspaceId}
      {@const ws = workspaces.find(w => w.id === activeWorkspaceId)}
      {#if ws}
        <div class="workspace-badge" style="border-color: {ws.color}; color: {ws.color}">
          ▦ {ws.name}
        </div>
      {/if}
    {/if}
  </div>

  <!-- ── Main Area ─────────────────────────────────────────────────────────── -->
  <div class="main">

    <!-- Content / New Tab Page -->
    <div class="content">
      {#if !activeTab?.url}
        <!-- New Tab Page -->
        <div class="newtab">
          <pre class="logo">rrsearch_</pre>
          <p class="tagline">go deeper.</p>
          <div class="quicklinks">
            <button onclick={() => { addressInput = "reddit.com"; navigate(); }}>reddit</button>
            <button onclick={() => { addressInput = "search.brave.com"; navigate(); }}>brave search</button>
            <button onclick={() => { addressInput = "github.com"; navigate(); }}>github</button>
            <button onclick={() => { addressInput = "news.ycombinator.com"; navigate(); }}>hackernews</button>
          </div>
          {#if sessions.length > 0}
            <div class="recent-sessions">
              <p class="section-label">// recent sessions</p>
              {#each sessions.slice(0, 4) as s}
                <div class="recent-session-item">
                  <span class="rs-name">{s.name}</span>
                  <span class="rs-date">{formatDate(s.started_at)}</span>
                  {#if s.decision_note}
                    <span class="rs-decision">→ {s.decision_note}</span>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
          <div class="start-session-hint">
            <button class="hint-btn" onclick={() => { showSessionPrompt = true; sessionPromptName = ""; }}>
              + start research session
            </button>
          </div>
        </div>
      {:else}
        <!-- Webview placeholder — real webview injected here by Tauri -->
        <div class="webview-placeholder">
          <p class="wv-url">{activeTab?.url}</p>
          <p class="wv-note">// webview renders here in desktop build</p>
          <p class="wv-note">// select text to clip it to your session</p>
        </div>
      {/if}
    </div>

    <!-- Sidebar -->
    {#if showSidebar}
      <div class="sidebar">
        <div class="sidebar-header">
          <button class="{sidebarTab === 'sessions' ? 'sb-tab-active' : 'sb-tab'}" onclick={() => (sidebarTab = 'sessions')}>sessions</button>
          <button class="{sidebarTab === 'clips' ? 'sb-tab-active' : 'sb-tab'}" onclick={() => { sidebarTab = 'clips'; loadClips(); }}>clips</button>
          <button class="{sidebarTab === 'workspaces' ? 'sb-tab-active' : 'sb-tab'}" onclick={() => (sidebarTab = 'workspaces')}>spaces</button>
          <button class="sidebar-close" onclick={() => (showSidebar = false)}>×</button>
        </div>

        {#if sidebarTab === 'sessions'}
          <div class="sb-content">
            <button class="sb-action-btn" onclick={() => { showSessionPrompt = true; sessionPromptName = ""; }}>+ new session</button>
            {#each sessions as s}
              <div class="sb-item {s.id === activeSessionId ? 'sb-item--active' : ''}">
                <div class="sb-item-name">{s.name}</div>
                <div class="sb-item-meta">{formatDate(s.started_at)}</div>
                {#if s.decision_note}
                  <div class="sb-item-decision">→ {s.decision_note}</div>
                {/if}
                {#if !s.ended_at}
                  <button class="sb-resume-btn" onclick={() => (activeSessionId = s.id)}>
                    {s.id === activeSessionId ? '● active' : 'resume'}
                  </button>
                {/if}
              </div>
            {/each}
          </div>
        {/if}

        {#if sidebarTab === 'clips'}
          <div class="sb-content">
            {#if clips.length === 0}
              <p class="sb-empty">// no clips yet<br/>select text on any page to clip it</p>
            {/if}
            {#each clips as clip}
              <div class="sb-clip">
                <div class="clip-text">"{clip.text}"</div>
                <div class="clip-source"><a href={clip.url} target="_blank">{clip.page_title}</a></div>
                <div class="clip-date">{formatDate(clip.clipped_at)}</div>
              </div>
            {/each}
          </div>
        {/if}

        {#if sidebarTab === 'workspaces'}
          <div class="sb-content">
            <button class="sb-action-btn" onclick={() => (showNewWorkspace = true)}>+ new workspace</button>
            <button class="sb-item {activeWorkspaceId === null ? 'sb-item--active' : ''}" onclick={() => (activeWorkspaceId = null)}>
              all tabs
            </button>
            {#each workspaces as ws}
              <button
                class="sb-item {ws.id === activeWorkspaceId ? 'sb-item--active' : ''}"
                onclick={() => (activeWorkspaceId = ws.id)}
                style="border-left: 3px solid {ws.color}"
              >
                <span class="sb-item-name">{ws.name}</span>
                <span class="sb-item-meta">{tabs.filter(t => t.workspaceId === ws.id).length} tabs</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- ── Terminal Drawer ───────────────────────────────────────────────────── -->
  {#if showTerminal}
    <div class="terminal">
      <div class="terminal-header">
        <span>rrsearch terminal</span>
        <button onclick={() => (showTerminal = false)}>×</button>
      </div>
      <div class="terminal-output">
        {#each terminalHistory as line}
          <div class="t-line">{line}</div>
        {/each}
      </div>
      <div class="terminal-input-row">
        <span class="t-prompt">❯</span>
        <input
          class="t-input"
          bind:value={terminalInput}
          onkeydown={handleTerminalKey}
          placeholder="command..."
          spellcheck="false"
          autocomplete="off"
        />
      </div>
    </div>
  {/if}

  <!-- ── Session Prompt Modal ──────────────────────────────────────────────── -->
  {#if showSessionPrompt}
    <div class="modal-overlay" onclick={() => (showSessionPrompt = false)} role="dialog" tabindex="-1" aria-label="New session">
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="modal" onclick={(e) => e.stopPropagation()}>
        <p class="modal-title">// start research session?</p>
        <p class="modal-sub">name this research bender to save all sites + clips</p>
        <input
          class="modal-input"
          bind:value={sessionPromptName}
          placeholder="session name..."
          onkeydown={(e) => e.key === 'Enter' && startSession()}
        />
        <div class="modal-actions">
          <button class="modal-btn-primary" onclick={startSession}>start</button>
          <button class="modal-btn-secondary" onclick={() => (showSessionPrompt = false)}>skip</button>
        </div>
      </div>
    </div>
  {/if}

  <!-- ── End Session Modal ─────────────────────────────────────────────────── -->
  {#if showEndSession}
    <div class="modal-overlay" onclick={() => (showEndSession = false)} role="dialog" tabindex="-1" aria-label="End session">
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="modal" onclick={(e) => e.stopPropagation()}>
        <p class="modal-title">// end session?</p>
        <p class="modal-sub">what did you decide? (optional)</p>
        <input
          class="modal-input"
          bind:value={sessionDecision}
          placeholder="your conclusion or decision..."
          onkeydown={(e) => e.key === 'Enter' && endSession()}
        />
        <div class="modal-actions">
          <button class="modal-btn-primary" onclick={endSession}>save + end</button>
          <button class="modal-btn-secondary" onclick={() => (showEndSession = false)}>cancel</button>
        </div>
      </div>
    </div>
  {/if}

  <!-- ── New Workspace Modal ───────────────────────────────────────────────── -->
  {#if showNewWorkspace}
    <div class="modal-overlay" onclick={() => (showNewWorkspace = false)} role="dialog" tabindex="-1" aria-label="New workspace">
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="modal" onclick={(e) => e.stopPropagation()}>
        <p class="modal-title">// new workspace</p>
        <input
          class="modal-input"
          bind:value={newWorkspaceName}
          placeholder="workspace name..."
          onkeydown={(e) => e.key === 'Enter' && createWorkspace()}
        />
        <div class="color-row">
          {#each ['#00ff41', '#ff6b35', '#4fc3f7', '#ce93d8', '#fff176', '#ef5350'] as c}
            <button
              class="color-swatch {newWorkspaceColor === c ? 'color-swatch--active' : ''}"
              style="background: {c}"
              onclick={() => (newWorkspaceColor = c)}
            ></button>
          {/each}
        </div>
        <div class="modal-actions">
          <button class="modal-btn-primary" onclick={createWorkspace}>create</button>
          <button class="modal-btn-secondary" onclick={() => (showNewWorkspace = false)}>cancel</button>
        </div>
      </div>
    </div>
  {/if}

</div>

<style>
  /* ── Reset & Root ──────────────────────────────────────────────────────── */
  :global(*, *::before, *::after) { box-sizing: border-box; margin: 0; padding: 0; }
  :global(body) {
    background: #0a0a0a;
    color: #e0e0e0;
    font-family: 'Space Mono', 'Courier New', monospace;
    font-size: 13px;
    overflow: hidden;
    height: 100vh;
    width: 100vw;
  }

  /* ── Browser Shell ─────────────────────────────────────────────────────── */
  .browser {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    background: #0a0a0a;
  }

  /* ── Tab Bar ───────────────────────────────────────────────────────────── */
  .tabbar {
    display: flex;
    align-items: center;
    background: #0f0f0f;
    border-bottom: 1px solid #1e1e1e;
    height: 36px;
    padding: 0 4px;
    gap: 4px;
    flex-shrink: 0;
  }
  .tabs-scroll {
    display: flex;
    align-items: center;
    gap: 2px;
    overflow-x: auto;
    flex: 1;
    scrollbar-width: none;
  }
  .tabs-scroll::-webkit-scrollbar { display: none; }
  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    height: 28px;
    background: #141414;
    border: 1px solid #1e1e1e;
    color: #666;
    cursor: pointer;
    white-space: nowrap;
    font-family: inherit;
    font-size: 12px;
    transition: all 0.1s;
    user-select: none;
    min-width: 80px;
    max-width: 180px;
  }
  .tab:hover { background: #1a1a1a; color: #999; }
  .tab--active {
    background: #0a0a0a;
    color: #00ff41;
    border-bottom-color: #0a0a0a;
    border-top-color: #00ff41;
  }
  .tab-title { flex: 1; overflow: hidden; text-overflow: ellipsis; }
  .tab-close {
    opacity: 0;
    color: #666;
    font-size: 14px;
    line-height: 1;
    padding: 0 2px;
    cursor: pointer;
  }
  .tab:hover .tab-close { opacity: 1; }
  .tab-close:hover { color: #ef5350; }
  .tab-new {
    background: none;
    border: 1px solid #1e1e1e;
    color: #444;
    width: 28px;
    height: 28px;
    cursor: pointer;
    font-size: 16px;
    font-family: inherit;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .tab-new:hover { color: #00ff41; border-color: #00ff41; }
  .tabbar-actions {
    display: flex;
    gap: 2px;
    flex-shrink: 0;
  }
  .icon-btn {
    background: none;
    border: 1px solid transparent;
    color: #444;
    width: 28px;
    height: 28px;
    cursor: pointer;
    font-family: inherit;
    font-size: 11px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .icon-btn:hover { color: #00ff41; border-color: #1e1e1e; }

  /* ── Address Bar ───────────────────────────────────────────────────────── */
  .addressbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    background: #0f0f0f;
    border-bottom: 1px solid #1e1e1e;
    flex-shrink: 0;
    height: 38px;
  }
  .address-form {
    display: flex;
    align-items: center;
    flex: 1;
    background: #141414;
    border: 1px solid #1e1e1e;
    padding: 0 8px;
    gap: 4px;
    height: 28px;
  }
  .address-form:focus-within { border-color: #00ff41; }
  .prompt { color: #333; font-size: 11px; white-space: nowrap; }
  .address-input {
    flex: 1;
    background: none;
    border: none;
    color: #e0e0e0;
    font-family: inherit;
    font-size: 12px;
    outline: none;
  }
  .address-input::placeholder { color: #333; }
  .go-btn {
    background: none;
    border: none;
    color: #444;
    cursor: pointer;
    font-family: inherit;
    font-size: 14px;
    padding: 0 4px;
  }
  .go-btn:hover { color: #00ff41; }
  .session-badge {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: #00ff41;
    background: #0a1a0a;
    border: 1px solid #00ff4133;
    padding: 2px 6px;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .session-dot { animation: blink 1.2s step-end infinite; }
  @keyframes blink { 0%, 100% { opacity: 1; } 50% { opacity: 0; } }
  .session-end-btn {
    background: none;
    border: none;
    color: #666;
    cursor: pointer;
    font-family: inherit;
    font-size: 10px;
    padding: 0 2px;
  }
  .session-end-btn:hover { color: #ef5350; }
  .workspace-badge {
    font-size: 11px;
    border: 1px solid;
    padding: 2px 6px;
    white-space: nowrap;
    flex-shrink: 0;
  }

  /* ── Main ──────────────────────────────────────────────────────────────── */
  .main {
    display: flex;
    flex: 1;
    overflow: hidden;
  }
  .content {
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  /* ── New Tab Page ──────────────────────────────────────────────────────── */
  .newtab {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 16px;
    padding: 40px;
  }
  .logo {
    font-size: 32px;
    font-weight: 700;
    color: #00ff41;
    letter-spacing: -1px;
    line-height: 1;
    text-shadow: 0 0 20px #00ff4166;
  }
  .tagline {
    color: #333;
    font-size: 12px;
    letter-spacing: 3px;
  }
  .quicklinks {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: center;
    margin-top: 8px;
  }
  .quicklinks button {
    background: #141414;
    border: 1px solid #1e1e1e;
    color: #666;
    padding: 6px 14px;
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .quicklinks button:hover { border-color: #00ff41; color: #00ff41; }
  .recent-sessions {
    width: 100%;
    max-width: 480px;
    margin-top: 8px;
  }
  .section-label { color: #333; font-size: 11px; margin-bottom: 8px; }
  .recent-session-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px;
    border: 1px solid #1a1a1a;
    margin-bottom: 4px;
    background: #0d0d0d;
  }
  .rs-name { color: #e0e0e0; font-size: 12px; }
  .rs-date { color: #444; font-size: 11px; }
  .rs-decision { color: #4fc3f7; font-size: 11px; font-style: italic; }
  .start-session-hint { margin-top: 4px; }
  .hint-btn {
    background: none;
    border: 1px dashed #222;
    color: #444;
    padding: 6px 16px;
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .hint-btn:hover { border-color: #00ff41; color: #00ff41; }

  /* ── Webview Placeholder ───────────────────────────────────────────────── */
  .webview-placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 12px;
  }
  .wv-url { color: #00ff41; font-size: 14px; }
  .wv-note { color: #333; font-size: 11px; }

  /* ── Sidebar ───────────────────────────────────────────────────────────── */
  .sidebar {
    width: 280px;
    background: #0d0d0d;
    border-left: 1px solid #1e1e1e;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }
  .sidebar-header {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 6px 8px;
    border-bottom: 1px solid #1e1e1e;
    flex-shrink: 0;
  }
  .sb-tab, .sb-tab-active {
    background: none;
    border: 1px solid transparent;
    color: #444;
    padding: 3px 8px;
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
    flex: 1;
  }
  .sb-tab-active { color: #00ff41; border-color: #1e1e1e; }
  .sb-tab:hover { color: #999; }
  .sidebar-close {
    background: none;
    border: none;
    color: #444;
    cursor: pointer;
    font-size: 16px;
    padding: 0 4px;
    margin-left: auto;
  }
  .sidebar-close:hover { color: #ef5350; }
  .sb-content {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    scrollbar-width: thin;
    scrollbar-color: #1e1e1e transparent;
  }
  .sb-action-btn {
    background: none;
    border: 1px dashed #222;
    color: #444;
    padding: 6px;
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
    text-align: left;
    margin-bottom: 4px;
  }
  .sb-action-btn:hover { border-color: #00ff41; color: #00ff41; }
  .sb-item {
    background: #0f0f0f;
    border: 1px solid #1a1a1a;
    padding: 8px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    text-align: left;
    font-family: inherit;
    font-size: 12px;
    color: #e0e0e0;
  }
  .sb-item:hover { border-color: #2e2e2e; }
  .sb-item--active { border-color: #00ff41; }
  .sb-item-name { color: #e0e0e0; font-size: 12px; }
  .sb-item-meta { color: #444; font-size: 10px; }
  .sb-item-decision { color: #4fc3f7; font-size: 10px; font-style: italic; }
  .sb-resume-btn {
    background: none;
    border: 1px solid #1e1e1e;
    color: #666;
    padding: 2px 6px;
    font-family: inherit;
    font-size: 10px;
    cursor: pointer;
    margin-top: 2px;
    align-self: flex-start;
  }
  .sb-resume-btn:hover { border-color: #00ff41; color: #00ff41; }
  .sb-empty { color: #333; font-size: 11px; line-height: 1.8; padding: 8px 0; }
  .sb-clip {
    background: #0f0f0f;
    border: 1px solid #1a1a1a;
    border-left: 3px solid #4fc3f7;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .clip-text { color: #c0c0c0; font-size: 11px; font-style: italic; line-height: 1.5; }
  .clip-source a { color: #4fc3f7; font-size: 10px; text-decoration: none; }
  .clip-source a:hover { text-decoration: underline; }
  .clip-date { color: #333; font-size: 10px; }
  .color-row { display: flex; gap: 6px; margin: 8px 0; }
  .color-swatch {
    width: 24px;
    height: 24px;
    border: 2px solid transparent;
    cursor: pointer;
  }
  .color-swatch--active { border-color: #fff; }

  /* ── Terminal ──────────────────────────────────────────────────────────── */
  .terminal {
    height: 220px;
    background: #080808;
    border-top: 1px solid #00ff4133;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }
  .terminal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 10px;
    border-bottom: 1px solid #1a1a1a;
    color: #00ff41;
    font-size: 11px;
    flex-shrink: 0;
  }
  .terminal-header button {
    background: none;
    border: none;
    color: #444;
    cursor: pointer;
    font-size: 14px;
  }
  .terminal-header button:hover { color: #ef5350; }
  .terminal-output {
    flex: 1;
    overflow-y: auto;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    scrollbar-width: thin;
    scrollbar-color: #1e1e1e transparent;
  }
  .t-line {
    font-size: 12px;
    color: #888;
    white-space: pre-wrap;
    line-height: 1.5;
  }
  .t-line:first-child { color: #00ff41; }
  .terminal-input-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-top: 1px solid #1a1a1a;
    flex-shrink: 0;
  }
  .t-prompt { color: #00ff41; font-size: 14px; }
  .t-input {
    flex: 1;
    background: none;
    border: none;
    color: #e0e0e0;
    font-family: inherit;
    font-size: 12px;
    outline: none;
  }
  .t-input::placeholder { color: #333; }

  /* ── Modals ────────────────────────────────────────────────────────────── */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: #000000bb;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .modal {
    background: #0f0f0f;
    border: 1px solid #2e2e2e;
    border-top: 2px solid #00ff41;
    padding: 24px;
    width: 380px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .modal-title { color: #00ff41; font-size: 13px; }
  .modal-sub { color: #555; font-size: 11px; }
  .modal-input {
    background: #141414;
    border: 1px solid #2e2e2e;
    color: #e0e0e0;
    padding: 8px 10px;
    font-family: inherit;
    font-size: 12px;
    outline: none;
    width: 100%;
  }
  .modal-input:focus { border-color: #00ff41; }
  .modal-actions { display: flex; gap: 8px; }
  .modal-btn-primary {
    background: #00ff41;
    color: #000;
    border: none;
    padding: 7px 16px;
    font-family: inherit;
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
  }
  .modal-btn-primary:hover { background: #00cc33; }
  .modal-btn-secondary {
    background: none;
    border: 1px solid #2e2e2e;
    color: #666;
    padding: 7px 16px;
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .modal-btn-secondary:hover { border-color: #444; color: #999; }
</style>

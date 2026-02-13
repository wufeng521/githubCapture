import { useState, useEffect, useRef, useMemo, useCallback, memo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Channel } from "@tauri-apps/api/core";

interface TrendingRepo {
  author: string;
  name: string;
  description: string;
  language: string;
  stars: string;
  forks: string;
  stars_today: string;
  url: string;
  topic: string;
  topics?: string[];
  pushed_at?: string;
  license?: string;
}

const MarkdownView = ({ content }: { content: string }) => {
  const parseMarkdown = (text: string) => {
    // 基础整理：处理换行
    let lines = text.split('\n');
    return lines.map((line, i) => {
      // 标题
      if (line.startsWith('### ')) return <h3 key={i} className="text-lg font-bold mt-4 mb-2 text-apple-text">{line.replace('### ', '')}</h3>;
      if (line.startsWith('## ')) return <h2 key={i} className="text-xl font-extrabold mt-6 mb-3 text-apple-text border-b border-apple-border/30 pb-2">{line.replace('## ', '')}</h2>;
      if (line.startsWith('# ')) return <h1 key={i} className="text-2xl font-black mt-8 mb-4 text-apple-text">{line.replace('# ', '')}</h1>;

      // 列表
      if (line.trim().startsWith('- ') || line.trim().startsWith('* ')) {
        return <li key={i} className="ml-4 mb-1 text-sm text-apple-secondary list-disc leading-relaxed pl-1">{
          line.trim().substring(2).split('**').map((part, j) => j % 2 === 1 ? <strong key={j} className="text-apple-text font-bold">{part}</strong> : part)
        }</li>;
      }

      // 普通段落处理加粗
      const parts = line.split('**');
      if (parts.length > 1) {
        return <p key={i} className="mb-3 text-sm text-apple-secondary leading-relaxed">{
          parts.map((part, j) => j % 2 === 1 ? <strong key={j} className="text-apple-text font-bold">{part}</strong> : part)
        }</p>;
      }

      return <p key={i} className={`text-sm text-apple-secondary leading-relaxed ${line.trim() === '' ? 'h-3' : 'mb-3'}`}>{line}</p>;
    });
  };

  return <div className="markdown-body">{parseMarkdown(content)}</div>;
};

type StreamPayload =
  | { type: "Token", data: string }
  | { type: "Error", data: string }
  | { type: "Done", data: null };

type ModelProvider =
  | "OpenAI"
  | "Anthropic"
  | "Google"
  | "DeepSeek"
  | "AzureOpenAI"
  | { Custom: string };

interface ModelConfig {
  id: string;
  name: string;
  provider: ModelProvider;
  api_base_url: string;
  api_key: string;
  default_model: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}


const TOPICS = ["All", "AI / LLM", "Web / App", "Tools / CLI", "Systems / OS", "Mobile", "General"];
const SINCE_OPTIONS = [
  { label: "Today", value: "daily" },
  { label: "Week", value: "weekly" },
  { label: "Month", value: "monthly" }
];

// --- Memoized Components ---

const Sidebar = memo(({ activeTab, setActiveTab, onSearchClick }: any) => (
  <aside className="w-64 border-r border-apple-border flex flex-col bg-white/50 backdrop-blur-md">
    <div className="p-6">
      <h1 className="text-xl font-semibold tracking-tight text-apple-text">GitHub Capture</h1>
    </div>
    <nav className="flex-1 px-3 space-y-1">
      <button
        onClick={() => setActiveTab("trending")}
        className={`w-full text-left rounded-md flex items-center px-4 py-2 transition-colors ${activeTab === "trending" ? "bg-black/5 text-apple-text shadow-sm" : "text-apple-secondary hover:bg-black/5"}`}
      >
        <span className="text-sm font-medium">热门榜单</span>
      </button>
      <button
        onClick={onSearchClick}
        className={`w-full text-left rounded-md flex items-center px-4 py-2 transition-colors ${activeTab === "search" ? "bg-black/5 text-apple-text shadow-sm" : "text-apple-secondary hover:bg-black/5"}`}
      >
        <span className="text-sm font-medium flex-1">智能搜索</span>
        <span className="text-[9px] bg-black/5 px-1 rounded opacity-50">⌘K</span>
      </button>
      <button
        onClick={() => setActiveTab("library")}
        className={`w-full text-left rounded-md flex items-center px-4 py-2 transition-colors ${activeTab === "library" ? "bg-black/5 text-apple-text shadow-sm" : "text-apple-secondary hover:bg-black/5"}`}
      >
        <span className="text-sm font-medium">本地收藏</span>
      </button>
    </nav>
    <div className="p-4 border-t border-apple-border">
      <button
        onClick={() => setActiveTab("settings")}
        className={`w-full rounded-md text-sm text-left px-4 py-2 transition-colors ${activeTab === "settings" ? "bg-black/5 text-apple-text shadow-sm" : "text-apple-secondary hover:bg-black/5"}`}
      >
        设置
      </button>
    </div>
  </aside>
));

const RepoList = memo(({ repos, selectedRepo, onSelectRepo, selectedTopic, setSelectedTopic, selectedSince, setSelectedSince, summarizedUrls, isLoading, activeTab, onRefresh }: any) => (
  <section className="w-[380px] border-r border-apple-border flex flex-col bg-white overflow-hidden">
    <header className="p-6 border-b border-apple-border/30">
      {activeTab === "trending" ? (
        <>
          <div className="flex items-center justify-between mb-6">
            <div className="flex bg-black/5 p-1 rounded-xl">
              {SINCE_OPTIONS.map(opt => (
                <button
                  key={opt.value}
                  onClick={() => setSelectedSince(opt.value)}
                  className={`px-3 py-1.5 rounded-lg text-[10px] font-bold transition-all ${selectedSince === opt.value ? "bg-white text-apple-text shadow-sm scale-105" : "text-apple-secondary hover:text-apple-text"}`}
                >
                  {opt.label}
                </button>
              ))}
            </div>
            <button onClick={onRefresh} className="p-2 bg-apple-accent text-white rounded-xl shadow-md hover:bg-blue-700 active:scale-95 transition-all">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                <path d="M21 2v6h-6M3 12a9 9 0 0115-6.7L21 8M3 22v-6h6m12-4a9 9 0 01-15 6.7L3 16" />
              </svg>
            </button>
          </div>

          <div className="flex space-x-2 overflow-x-auto pb-1 no-scrollbar">
            {TOPICS.map(topic => (
              <button
                key={topic}
                onClick={() => setSelectedTopic(topic)}
                className={`px-3 py-1.5 rounded-full text-[10px] font-bold whitespace-nowrap transition-all border ${selectedTopic === topic ? "bg-black text-white border-black" : "bg-white text-apple-secondary border-apple-border/50 hover:border-apple-accent/30"}`}
              >
                {topic}
              </button>
            ))}
          </div>
        </>
      ) : (
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-black text-apple-text uppercase tracking-widest">
            {activeTab === "library" ? "本地收藏" : "智能搜索"}
          </h3>
          <span className="text-[10px] text-apple-secondary bg-black/5 px-2 py-0.5 rounded-full font-bold">
            {repos.length} 个项目
          </span>
        </div>
      )}
    </header>

    <div className="flex-1 overflow-y-auto p-4 space-y-2 bg-apple-bg/5 scroll-smooth">
      {isLoading ? (
        <div className="flex flex-col items-center justify-center py-20 space-y-4">
          <div className="w-8 h-8 border-4 border-apple-accent/20 border-t-apple-accent rounded-full animate-spin"></div>
          <span className="text-xs text-apple-secondary font-medium">同步 GitHub 趋势中...</span>
        </div>
      ) : (
        repos.map((repo: any) => (
          <div
            key={repo.url}
            onClick={() => onSelectRepo(repo)}
            className={`p-5 rounded-2xl cursor-pointer transition-all border ${selectedRepo?.url === repo.url ? "bg-white border-apple-accent shadow-md ring-4 ring-apple-accent/5 scale-[1.02] z-10" : "bg-white/80 border-apple-border/30 hover:border-apple-accent/20 hover:bg-white"}`}
          >
            <div className="flex items-center justify-between mb-2">
              <span className="px-1.5 py-0.5 rounded text-[8px] font-bold bg-apple-accent/10 text-apple-accent uppercase tracking-tighter">
                {repo.topic}
              </span>
              {summarizedUrls?.has(repo.url) && (
                <span className="flex items-center space-x-1 text-apple-accent bg-apple-accent/5 px-1.5 py-0.5 rounded-md border border-apple-accent/10" title="已有 AI 洞察分析">
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M12 2L4.5 20.29l.71.71L12 18l6.79 3 .71-.71L12 2z" />
                  </svg>
                  <span className="text-[9px] font-bold">AI已总结</span>
                </span>
              )}
            </div>
            <div className="font-bold text-sm text-apple-text tracking-tight leading-tight">
              {repo.author} / {repo.name}
            </div>
            <div className="text-[11px] text-apple-secondary line-clamp-2 mt-2 leading-relaxed opacity-80">{repo.description || "No description provided."}</div>

            <div className="flex items-center justify-between mt-4">
              <div className="flex items-center space-x-4 text-[10px] text-apple-secondary font-bold font-sans">
                <div className="flex items-center">
                  <span className="w-1.5 h-1.5 rounded-full bg-apple-accent mr-1.5"></span>
                  {repo.language}
                </div>
                <div className="flex items-center">
                  <span className="mr-1 opacity-60">★</span>
                  {repo.stars}
                </div>
                <div className="flex items-center">
                  <span className="mr-1 opacity-40">⑂</span>
                  {repo.forks}
                </div>
              </div>

              <div className="flex -space-x-1.5">
                {repo.built_by?.slice(0, 5).map((avatar: string, i: number) => (
                  <img
                    key={i}
                    src={avatar}
                    className="w-5 h-5 rounded-full border border-white ring-1 ring-black/5"
                    alt="contributor"
                  />
                ))}
              </div>
            </div>

            <div className="mt-2.5 pt-2.5 border-t border-black/5 flex items-center justify-end">
              <span className="text-[9px] font-black text-apple-accent flex items-center">
                <svg width="8" height="8" viewBox="0 0 24 24" fill="currentColor" className="mr-0.5">
                  <path d="M12 2L4.5 20.29l.71.71L12 18l6.79 3 .71-.71L12 2z" />
                </svg>
                {repo.stars_today}
              </span>
            </div>
          </div>
        ))
      )}
    </div>
  </section>
));

const InsightPanel = memo(({
  selectedRepo,
  insight,
  isSummarizing,
  apiKey,
  onSummarize,
  onSettingsClick,
  deepContextEnabled,
  setDeepContextEnabled,
  activeTab,
  selectedSince,
  isFavorite,
  onToggleFavorite
}: any) => (
  <section className="flex-1 flex flex-col overflow-hidden bg-white">
    {selectedRepo ? (
      <>
        <header className="p-8 border-b border-apple-border bg-apple-bg/10">
          <div className="flex justify-between items-start max-w-4xl mx-auto w-full">
            <div className="flex-1">
              <div className="flex items-center space-x-2 mb-3">
                <span className={`px-2 py-0.5 rounded text-[10px] font-bold uppercase tracking-widest ${selectedRepo.topic === "Search Result" ? "bg-apple-accent text-white" : "bg-black/5 text-apple-secondary"}`}>
                  {selectedRepo.topic}
                </span>
                {activeTab === "trending" && <span className="text-apple-accent text-[10px] font-extrabold font-sans">TRENDING {selectedSince.toUpperCase()}</span>}
              </div>
              <h2 className="text-3xl font-extrabold tracking-tight text-apple-text mb-3 leading-tight">{selectedRepo.author} / {selectedRepo.name}</h2>
              <div className="flex items-center space-x-4 text-xs text-apple-secondary font-medium font-sans">
                <span className="text-apple-text font-bold px-2 py-0.5 bg-white rounded-md shadow-sm">{selectedRepo.language}</span>
                <span>•</span>
                <span>{selectedRepo.stars} stars</span>
                <span>•</span>
                <span>{selectedRepo.forks} forks</span>
                {selectedRepo.license && selectedRepo.license !== "None" && (
                  <>
                    <span>•</span>
                    <span className="flex items-center">
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="mr-1 opacity-70">
                        <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
                      </svg>
                      {selectedRepo.license}
                    </span>
                  </>
                )}
                {selectedRepo.pushed_at && (
                  <>
                    <span>•</span>
                    <span className="flex items-center">
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="mr-1 opacity-70">
                        <path d="M21 12a9 9 0 1 1-6.219-8.56" />
                        <polyline points="12 7 12 12 15 15" />
                      </svg>
                      {new Date(selectedRepo.pushed_at).toLocaleDateString()}
                    </span>
                  </>
                )}
              </div>
              {selectedRepo.topics && selectedRepo.topics.length > 0 && (
                <div className="flex flex-wrap gap-2 mt-4">
                  {selectedRepo.topics.map((topic: string) => (
                    <span key={topic} className="px-2 py-0.5 bg-black/5 text-apple-secondary text-[10px] font-bold rounded-md border border-black/5">
                      #{topic}
                    </span>
                  ))}
                </div>
              )}
            </div>
            <div className="flex space-x-3 mt-2">
              <button
                onClick={() => onToggleFavorite(selectedRepo)}
                className={`p-2.5 bg-white border border-apple-border rounded-xl transition-all shadow-sm ${isFavorite ? "text-apple-accent border-apple-accent/30 bg-apple-accent/5" : "text-apple-secondary hover:bg-apple-bg"}`}
                title={isFavorite ? "取消收藏" : "收藏项目"}
              >
                <svg width="18" height="18" viewBox="0 0 24 24" fill={isFavorite ? "currentColor" : "none"} stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z" />
                </svg>
              </button>
              <a href={selectedRepo.url} target="_blank" rel="noopener noreferrer" className="px-6 py-2.5 bg-black text-white rounded-xl text-xs font-bold hover:shadow-xl hover:bg-black/80 transition-all flex items-center space-x-2 font-sans">
                <span>GitHub</span>
              </a>
            </div>
          </div>
        </header>
        <div className="flex-1 p-10 overflow-y-auto">
          <div className="max-w-4xl mx-auto">
            <div className="flex items-center space-x-3 mb-8">
              <div className="px-3 py-1 bg-apple-accent text-white rounded-full text-[10px] font-extrabold uppercase tracking-widest font-sans">
                AI Deep Insight
              </div>
              {isSummarizing && (
                <span className="flex items-center space-x-1 ml-4 bg-apple-accent/5 px-3 py-1 rounded-full border border-apple-accent/10 font-sans">
                  <span className="text-[10px] font-bold text-apple-accent/60 uppercase animate-pulse">Analysing</span>
                </span>
              )}
              {!isSummarizing && insight && (
                <div className="flex items-center space-x-4 ml-auto">
                  <div className="flex items-center space-x-2 bg-apple-accent/5 px-3 py-1.5 rounded-xl border border-apple-accent/10">
                    <button
                      onClick={() => setDeepContextEnabled(!deepContextEnabled)}
                      className={`relative w-8 h-4 rounded-full transition-colors duration-200 ${deepContextEnabled ? "bg-apple-accent" : "bg-black/10"}`}
                    >
                      <span className={`absolute top-0.5 left-0.5 w-3 h-3 bg-white rounded-full shadow-md transition-transform duration-200 ${deepContextEnabled ? "translate-x-4" : "translate-x-0"}`} />
                    </button>
                    <span className="text-[10px] font-bold text-apple-text opacity-70">深度模式</span>
                  </div>
                  <button onClick={() => onSummarize(selectedRepo, true)} className="flex items-center space-x-1.5 text-[10px] font-bold text-apple-accent hover:opacity-70 transition-all font-sans">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M21 2v6h-6M3 12a9 9 0 0115-6.7L21 8M3 22v-6h6m12-4a9 9 0 01-15 6.7L3 16" />
                    </svg>
                    <span>重新生成</span>
                  </button>
                </div>
              )}
            </div>

            {!isSummarizing && !insight && (
              <div className="flex items-center p-4 mb-6 bg-apple-accent/5 rounded-2xl border border-apple-accent/10">
                <button
                  onClick={() => setDeepContextEnabled(!deepContextEnabled)}
                  className={`relative w-10 h-5 rounded-full transition-colors duration-200 ${deepContextEnabled ? "bg-apple-accent" : "bg-black/10"}`}
                >
                  <span className={`absolute top-0.5 left-0.5 w-4 h-4 bg-white rounded-full shadow-md transition-transform duration-200 ${deepContextEnabled ? "translate-x-5" : "translate-x-0"}`} />
                </button>
                <div className="ml-3">
                  <span className="text-[12px] font-bold text-apple-text">启用深度上下文分析</span>
                  <p className="text-[10px] text-apple-secondary opacity-70">将抓取目录结构和配置文件，通过更多 Token 提供精准架构分析</p>
                </div>
              </div>
            )}

            <article>
              {insight ? (
                <div className="text-[15px] leading-relaxed text-apple-text bg-apple-bg/5 p-8 rounded-3xl border border-apple-border/40 selection:bg-apple-accent/20">
                  <MarkdownView content={insight} />
                </div>
              ) : (
                <div className="space-y-8">
                  {!apiKey ? (
                    <div className="flex flex-col items-center justify-center py-32 bg-apple-bg/20 rounded-3xl border border-dashed border-apple-border">
                      <p className="text-apple-text text-sm font-bold tracking-tight">配置 API Key 开启技术洞察</p>
                      <button onClick={onSettingsClick} className="mt-6 px-6 py-2.5 bg-apple-accent text-white text-[11px] font-extrabold rounded-full hover:shadow-lg transition-all font-sans">前往设置 →</button>
                    </div>
                  ) : (
                    <div className="flex flex-col items-center justify-center py-20 bg-apple-bg/20 rounded-3xl border border-dashed border-apple-border">
                      <p className="text-apple-text text-sm font-bold tracking-tight">点击生成 AI 深度洞察</p>
                      <button onClick={() => onSummarize(selectedRepo)} disabled={isSummarizing} className="mt-6 px-6 py-2.5 bg-apple-accent text-white text-[11px] font-extrabold rounded-full hover:shadow-lg transition-all disabled:opacity-50 font-sans">
                        {isSummarizing ? "正在分析..." : "生成 AI 洞察 ✨"}
                      </button>
                    </div>
                  )}
                </div>
              )}
            </article>
          </div>
        </div>
      </>
    ) : (
      <div className="flex-1 flex flex-col items-center justify-center text-apple-secondary space-y-8 bg-apple-bg/10">
        <h3 className="text-base font-black text-apple-text mb-2 tracking-tight">探索 GitHub 趋势</h3>
      </div>
    )}
  </section>
));

function App() {
  const [activeTab, setActiveTab] = useState("trending");
  const [selectedTopic, setSelectedTopic] = useState("All");
  const [selectedSince, setSelectedSince] = useState("daily");
  const [repos, setRepos] = useState<TrendingRepo[]>([]);
  const [selectedRepo, setSelectedRepo] = useState<TrendingRepo | null>(null);
  const [_error, setError] = useState<string | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [insight, setInsight] = useState("");
  const [isSummarizing, setIsSummarizing] = useState(false);
  const [isLoadingTrending, setIsLoadingTrending] = useState(false);

  // Model Config State
  const [modelConfigs, setModelConfigs] = useState<ModelConfig[]>([]);
  const [activeConfigId, setActiveConfigId] = useState<string | null>(null);
  const [isEditingConfig, setIsEditingConfig] = useState(false);
  const [editingConfig, setEditingConfig] = useState<Partial<ModelConfig> | null>(null);
  const [isTestingConnection, setIsTestingConnection] = useState(false);
  const [testResult, setTestResult] = useState<{ success: boolean; message: string } | null>(null);

  // Search State
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<TrendingRepo[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);

  // AI Rewrite State
  const [aiRewriteEnabled, setAiRewriteEnabled] = useState(true);
  const [rewrittenQuery, setRewrittenQuery] = useState("");
  const [isRewriting, setIsRewriting] = useState(false);
  const [rewriteError, setRewriteError] = useState<string | null>(null);
  const [deepContextEnabled, setDeepContextEnabled] = useState(false);
  const [summarizedUrls, setSummarizedUrls] = useState<Set<string>>(new Set());
  const [favoriteRepos, setFavoriteRepos] = useState<TrendingRepo[]>([]);
  const [isFavorite, setIsFavorite] = useState(false);

  const insightRef = useRef("");
  const searchInputRef = useRef<HTMLInputElement>(null);
  const isComposingRef = useRef(false);

  useEffect(() => {
    initStore();

    // Global shortcut listener (⌘K / Ctrl+K)
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setActiveTab("search");
        setTimeout(() => searchInputRef.current?.focus(), 100);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  useEffect(() => {
    if (activeTab === "trending") {
      fetchTrending();
    } else if (activeTab === "library") {
      fetchFavorites();
    }
  }, [activeTab, selectedSince]);

  useEffect(() => {
    // 批量检查已有洞察的项目
    const currentRepos = activeTab === "trending" ? repos : (activeTab === "search" ? searchResults : favoriteRepos);
    if (currentRepos.length > 0) {
      checkInsightsBatch(currentRepos);
    }
  }, [repos, searchResults, favoriteRepos, activeTab]);

  useEffect(() => {
    if (selectedRepo) {
      // 切换项目时，尝试先加载缓存
      setInsight("");
      insightRef.current = "";
      checkCache(selectedRepo);
      checkFavoriteStatus(selectedRepo.url);
    }
  }, [selectedRepo]);

  const checkCache = async (repo: TrendingRepo) => {
    try {
      const cached: string | null = await invoke("get_cached_insight", { repo });
      if (cached) {
        setInsight(cached);
        insightRef.current = cached;
      }
    } catch (e) {
      console.error("Check cache failed:", e);
    }
  };

  const initStore = async () => {
    try {
      // 1. 尝试从配置管理器加载所有配置
      const configs: ModelConfig[] = await invoke("get_model_configs");
      setModelConfigs(configs);

      // 2. 加载当前激活的配置
      const active: ModelConfig | null = await invoke("get_active_model_config");
      if (active) {
        setActiveConfigId(active.id);
        setApiKey(active.api_key); // 为旧代码保留兼容性
      }

      // 3. 回退逻辑：如果模型管理器没配置但 store 里有旧的 key，后端应该已经自动迁移了
      // 这里的 initStore 主要是为了前端状态同步
    } catch (e) {
      console.error("Store init failed:", e);
    }
  };

  const handleSetActiveConfig = async (id: string) => {
    try {
      await invoke("set_active_model_config", { configId: id });
      setActiveConfigId(id);
      const active = modelConfigs.find(c => c.id === id);
      if (active) {
        setApiKey(active.api_key);
        alert(`已成功激活配置: ${active.name}`);
      }
    } catch (e) {
      console.error("Set active config failed:", e);
    }
  };

  const handleDeleteConfig = async (id: string) => {
    try {
      await invoke("delete_model_config", { configId: id });
      setModelConfigs(prev => prev.filter(c => c.id !== id));
      if (activeConfigId === id) {
        setActiveConfigId(null);
        setApiKey("");
      }
      alert("配置已删除");
    } catch (e: any) {
      console.error("Delete config failed:", e);
      alert("删除失败: " + e);
    }
  };

  const handleSaveConfig = async (config: ModelConfig) => {
    try {
      await invoke("save_model_config", { config });
      await initStore(); // 重新加载
      setIsEditingConfig(false);
      setEditingConfig(null);
    } catch (e) {
      console.error("Save config failed:", e);
      alert("保存失败: " + e);
    }
  };

  const handleUpdateConfig = async (id: string, updates: any) => {
    try {
      await invoke("update_model_config", { configId: id, updates });
      await initStore();
      setIsEditingConfig(false);
      setEditingConfig(null);
    } catch (e) {
      console.error("Update config failed:", e);
      alert("更新失败: " + e);
    }
  };

  const handleTestConnection = async (id: string) => {
    setIsTestingConnection(true);
    setTestResult(null);
    try {
      await invoke("test_model_connection", { modelConfigId: id });
      setTestResult({ success: true, message: "连接成功！" });
    } catch (e: any) {
      setTestResult({ success: false, message: "连接失败: " + e });
    } finally {
      setIsTestingConnection(false);
    }
  };

  const fetchTrending = async () => {
    setError(null);
    setIsLoadingTrending(true);
    try {
      const result: TrendingRepo[] = await invoke("get_trending", {
        language: null,
        since: selectedSince
      });
      setRepos(result);
      if (result.length > 0) {
        if (!selectedRepo || !result.find(r => r.url === selectedRepo.url)) {
          setSelectedRepo(result[0]);
        }
      }
    } catch (error: any) {
      console.error("Failed to fetch trending:", error);
      setError(error.toString());
    } finally {
      setIsLoadingTrending(false);
    }
  };

  // 执行搜索
  const handleSearch = async () => {
    const finalQuery = aiRewriteEnabled && rewrittenQuery ? rewrittenQuery : searchQuery;
    if (!finalQuery.trim()) return;
    setIsSearching(true);
    setSearchError(null);
    try {
      const result: TrendingRepo[] = await invoke("search_github", {
        query: finalQuery
      });
      setSearchResults(result);
      if (result.length > 0) {
        setSelectedRepo(result[0]);
      }
    } catch (e: any) {
      console.error("Search failed:", e);
      setSearchError(e.toString());
    } finally {
      setIsSearching(false);
    }
  };

  // 一键搜索：若开启 AI 先改写再搜索，否则直接搜索
  const handleFullSearch = async () => {
    if (!searchQuery.trim()) return;
    if (aiRewriteEnabled && apiKey) {
      setIsRewriting(true);
      setRewriteError(null);
      setRewrittenQuery("");
      try {
        const rewritten: string = await invoke("ai_rewrite_query", {
          query: searchQuery,
          modelConfigId: activeConfigId,
          apiKey: !activeConfigId ? apiKey : undefined
        });
        setRewrittenQuery(rewritten);
        // 自动使用改写后的查询进行搜索
        setIsSearching(true);
        setSearchError(null);
        const result: TrendingRepo[] = await invoke("search_github", {
          query: rewritten
        });
        setSearchResults(result);
        if (result.length > 0) {
          setSelectedRepo(result[0]);
        }
      } catch (e: any) {
        console.error("Full search failed:", e);
        setSearchError(e.toString());
      } finally {
        setIsRewriting(false);
        setIsSearching(false);
      }
    } else {
      await handleSearch();
    }
  };

  const filteredRepos = useMemo(() => {
    if (selectedTopic === "All") return repos;
    return repos.filter(r => r.topic === selectedTopic || r.topic === "Search Result");
  }, [repos, selectedTopic]);

  const handleSummarize = useCallback(async (repo: TrendingRepo, forceRefresh: boolean = false) => {
    setInsight("");
    insightRef.current = "";
    setIsSummarizing(true);

    const onEvent = new Channel<StreamPayload>();
    onEvent.onmessage = (payload) => {
      if (payload.type === "Token") {
        insightRef.current += payload.data;
        setInsight(insightRef.current);
      } else if (payload.type === "Done") {
        setIsSummarizing(false);
      } else if (payload.type === "Error") {
        setInsight(prev => prev + `\n\n[Error: ${payload.data}]`);
        setIsSummarizing(false);
      }
    };

    try {
      await invoke("summarize_repo", {
        repo: {
          author: repo.author,
          name: repo.name,
          description: repo.description,
          language: repo.language,
          url: repo.url
        },
        modelConfigId: activeConfigId,
        apiKey: !activeConfigId ? apiKey : undefined,
        deepContext: deepContextEnabled,
        forceRefresh,
        onEvent
      });
    } catch (error) {
      console.error("Summarize failed:", error);
      setIsSummarizing(false);
    } finally {
      // 成功生成后刷新洞察状态
      checkInsightsBatch([repo]);
    }
  }, [activeConfigId, apiKey, deepContextEnabled]);

  const fetchFavorites = async () => {
    try {
      const result: TrendingRepo[] = await invoke("get_favorites");
      setFavoriteRepos(result);
    } catch (e) {
      console.error("Fetch favorites failed:", e);
    }
  };

  const checkFavoriteStatus = async (url: string) => {
    try {
      const favorited: boolean = await invoke("is_favorite", { url });
      setIsFavorite(favorited);
    } catch (e) {
      console.error("Check favorite status failed:", e);
    }
  };

  const handleToggleFavorite = useCallback(async (repo: TrendingRepo) => {
    try {
      const nowFavorited: boolean = await invoke("toggle_favorite", {
        repo: {
          author: repo.author,
          name: repo.name,
          description: repo.description,
          language: repo.language,
          url: repo.url,
          stars: repo.stars,
          forks: repo.forks
        }
      });
      setIsFavorite(nowFavorited);
      // 刷新收藏列表
      if (activeTab === "library") {
        fetchFavorites();
      }
    } catch (e: any) {
      console.error("Toggle favorite failed:", e);
      alert("收藏操作失败: " + e.toString());
    }
  }, [activeTab]);

  const checkInsightsBatch = async (repoList: TrendingRepo[]) => {
    try {
      const existingUrls: string[] = await invoke("check_insights_batch", {
        repos: repoList.map(r => ({
          author: r.author,
          name: r.name,
          description: r.description,
          language: r.language,
          url: r.url
        }))
      });
      setSummarizedUrls(prev => {
        const next = new Set(prev);
        existingUrls.forEach(url => next.add(url));
        return next;
      });
    } catch (e) {
      console.error("Check insights batch failed:", e);
    }
  };

  const handleSettingsClick = useCallback(() => setActiveTab("settings"), []);
  const handleSearchClick = useCallback(() => {
    setActiveTab("search");
    setTimeout(() => searchInputRef.current?.focus(), 100);
  }, []);
  const handleSelectRepo = useCallback((repo: TrendingRepo) => setSelectedRepo(repo), []);

  // ============ 搜索面板渲染 ============
  const renderSearchPanel = () => (
    <section className="flex-1 flex flex-col overflow-hidden bg-white">
      {/* 搜索输入区 */}
      <div className="p-8 border-b border-apple-border/30 bg-apple-bg/5">
        <div className="max-w-3xl mx-auto">
          {/* 标题 */}
          <div className="flex items-center space-x-3 mb-6">
            <div className="w-10 h-10 bg-apple-accent rounded-2xl flex items-center justify-center shadow-md">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="white" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="11" cy="11" r="8" />
                <path d="m21 21-4.3-4.3" />
              </svg>
            </div>
            <div>
              <h2 className="text-xl font-bold tracking-tight text-apple-text">智能搜索</h2>
              <p className="text-[11px] text-apple-secondary mt-0.5">输入自然语言描述或关键词，精准发现全球开源项目</p>
            </div>
          </div>

          {/* 搜索框 + 按钮 */}
          <div className="flex space-x-3">
            <div className="flex-1 relative">
              <div className="absolute left-4 top-1/2 -translate-y-1/2 text-apple-secondary/40">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  <circle cx="11" cy="11" r="8" />
                  <path d="m21 21-4.3-4.3" />
                </svg>
              </div>
              <input
                ref={searchInputRef}
                autoFocus
                value={searchQuery}
                onChange={(e) => {
                  setSearchQuery(e.target.value);
                  setRewrittenQuery("");
                  setRewriteError(null);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !isComposingRef.current) {
                    handleFullSearch();
                  }
                }}
                onCompositionStart={() => {
                  isComposingRef.current = true;
                }}
                onCompositionEnd={() => {
                  isComposingRef.current = false;
                }}
                placeholder="例如：适合初学者的 Rust AI 框架、最近火爆的 React UI 库..."
                className="w-full pl-12 pr-4 py-4 text-[15px] bg-white border border-apple-border rounded-2xl outline-none focus:ring-2 focus:ring-apple-accent/20 focus:border-apple-accent/30 font-medium placeholder:text-apple-secondary/40 shadow-sm transition-all select-text"
              />
            </div>
            <button
              onClick={handleFullSearch}
              disabled={isSearching || isRewriting || !searchQuery.trim()}
              className="px-8 py-4 bg-apple-accent text-white rounded-2xl text-sm font-bold shadow-md hover:shadow-lg hover:bg-blue-700 active:scale-[0.98] disabled:opacity-40 disabled:cursor-not-allowed transition-all flex items-center space-x-2 whitespace-nowrap"
            >
              {isSearching || isRewriting ? (
                <>
                  <svg className="animate-spin" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M21 12a9 9 0 1 1-6.219-8.56" />
                  </svg>
                  <span>{isRewriting ? "AI 分析中..." : "搜索中..."}</span>
                </>
              ) : (
                <>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                    <circle cx="11" cy="11" r="8" />
                    <path d="m21 21-4.3-4.3" />
                  </svg>
                  <span>搜索</span>
                </>
              )}
            </button>
          </div>

          {/* AI 改写开关区域 */}
          <div className="mt-4 flex items-center justify-between">
            <div className="flex items-center space-x-3">
              {/* Toggle Switch */}
              <button
                onClick={() => {
                  setAiRewriteEnabled(!aiRewriteEnabled);
                  setRewrittenQuery("");
                  setRewriteError(null);
                }}
                className={`relative w-11 h-6 rounded-full transition-colors duration-200 ${aiRewriteEnabled ? "bg-apple-accent" : "bg-black/10"}`}
              >
                <span className={`absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow-md transition-transform duration-200 ${aiRewriteEnabled ? "translate-x-5" : "translate-x-0"}`} />
              </button>
              <div className="flex items-center space-x-1.5">
                <span className="text-sm font-medium text-apple-text">AI 智能改写</span>
                <span className="text-[10px] text-apple-secondary bg-black/5 px-1.5 py-0.5 rounded-md font-medium">
                  {aiRewriteEnabled ? "已开启" : "已关闭"}
                </span>
              </div>
            </div>
            {!apiKey && aiRewriteEnabled && (
              <button
                onClick={() => setActiveTab("settings")}
                className="text-[11px] text-apple-accent font-semibold hover:underline"
              >
                需要配置 API Key →
              </button>
            )}
          </div>

          {/* AI 改写预览 */}
          {aiRewriteEnabled && rewrittenQuery && (
            <div className="mt-4 bg-apple-accent/5 border border-apple-accent/15 rounded-xl p-4">
              <div className="flex items-center space-x-2 mb-2">
                <span className="text-[10px] font-bold text-apple-accent uppercase tracking-widest">✨ AI 优化查询</span>
              </div>
              <div className="flex items-center space-x-3">
                <code className="flex-1 text-sm text-apple-text bg-white/80 px-3 py-2 rounded-lg border border-apple-border/20 font-mono">
                  {rewrittenQuery}
                </code>
                <button
                  onClick={handleSearch}
                  className="px-4 py-2 text-[11px] font-bold text-apple-accent border border-apple-accent/30 rounded-lg hover:bg-apple-accent/5 transition-all whitespace-nowrap"
                >
                  使用此查询
                </button>
              </div>
            </div>
          )}

          {/* 改写中的加载状态 */}
          {isRewriting && (
            <div className="mt-4 flex items-center space-x-3 text-apple-accent">
              <div className="w-4 h-4 border-2 border-apple-accent/20 border-t-apple-accent rounded-full animate-spin" />
              <span className="text-[12px] font-semibold">AI 正在理解您的搜索意图并优化查询条件...</span>
            </div>
          )}

          {/* 错误提示 */}
          {(searchError || rewriteError) && (
            <div className="mt-4 bg-red-50 border border-red-200 rounded-xl p-3 text-sm text-red-600">
              {rewriteError || searchError}
            </div>
          )}
        </div>
      </div>

      {/* 搜索结果区 */}
      <div className="flex-1 flex overflow-hidden">
        {searchResults.length > 0 ? (
          <div className={`flex-1 flex overflow-hidden ${selectedRepo ? "bg-white" : ""}`}>
            {/* 结果列表 - 选中项目后变为侧边栏风格 */}
            <div className={`${selectedRepo ? "w-[380px] border-r border-apple-border flex flex-col" : "w-full overflow-y-auto"}`}>
              <div className={`${selectedRepo ? "p-4 space-y-2 overflow-y-auto h-full" : "max-w-3xl mx-auto p-8 space-y-3"}`}>
                <div className="flex items-center justify-between mb-4 px-1">
                  <div className="flex items-center space-x-2">
                    <span className="text-[10px] font-black text-apple-accent uppercase tracking-widest">搜索结果</span>
                    <span className="text-[10px] text-apple-secondary bg-black/5 px-2 py-0.5 rounded-full font-bold">
                      {searchResults.length}
                    </span>
                  </div>
                </div>

                {searchResults.map((repo) => (
                  <div
                    key={repo.url}
                    onClick={() => handleSelectRepo(repo)}
                    className={`rounded-2xl cursor-pointer transition-all border ${selectedRepo?.url === repo.url
                      ? "bg-apple-accent/5 border-apple-accent/20 shadow-md p-4"
                      : selectedRepo
                        ? "p-4 border-transparent hover:bg-black/5"
                        : "p-6 bg-white border-apple-border/30 hover:border-apple-accent/20 hover:shadow-sm"
                      }`}
                  >
                    <div className="flex flex-col min-w-0">
                      <div className="flex items-start justify-between mb-2">
                        <div className="flex items-center space-x-2">
                          <span className="px-1.5 py-0.5 rounded text-[8px] font-bold bg-apple-accent/10 text-apple-accent uppercase">
                            {repo.language}
                          </span>
                          {summarizedUrls?.has(repo.url) && (
                            <span className="flex items-center space-x-1 text-apple-accent" title="已有 AI 洞察分析">
                              <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor">
                                <path d="M12 2L4.5 20.29l.71.71L12 18l6.79 3 .71-.71L12 2z" />
                              </svg>
                            </span>
                          )}
                        </div>
                        <div className="flex items-center space-x-3 text-[10px] text-apple-secondary font-bold shrink-0">
                          <span className="flex items-center">
                            <span className="mr-1 opacity-60">★</span>
                            {repo.stars}
                          </span>
                          <span className="flex items-center">
                            <span className="mr-1 opacity-40">⑂</span>
                            {repo.forks}
                          </span>
                        </div>
                      </div>

                      <div className={`font-black text-apple-text tracking-tight truncate ${selectedRepo ? "text-[13px]" : "text-base"}`}>
                        {repo.author} / {repo.name}
                      </div>

                      <div className={`text-apple-secondary mt-1.5 leading-relaxed opacity-80 ${selectedRepo ? "text-[10px] line-clamp-1" : "text-[12px] line-clamp-2"}`}>
                        {repo.description || "No description provided."}
                      </div>

                      {repo.topics && repo.topics.length > 0 && (
                        <div className="flex flex-wrap gap-1.5 mt-3">
                          {repo.topics.slice(0, selectedRepo ? 2 : 4).map((topic) => (
                            <span key={topic} className="px-2 py-0.5 bg-black/5 text-[9px] font-bold text-apple-secondary rounded-full border border-black/5">
                              {topic}
                            </span>
                          ))}
                          {repo.topics.length > (selectedRepo ? 2 : 4) && (
                            <span className="text-[9px] text-apple-secondary/50 font-bold self-center">
                              +{repo.topics.length - (selectedRepo ? 2 : 4)}
                            </span>
                          )}
                        </div>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* 选中项目的详情面板 */}
            {selectedRepo && (
              <InsightPanel
                selectedRepo={selectedRepo}
                insight={insight}
                isSummarizing={isSummarizing}
                apiKey={apiKey}
                onSummarize={handleSummarize}
                onSettingsClick={handleSettingsClick}
                deepContextEnabled={deepContextEnabled}
                setDeepContextEnabled={setDeepContextEnabled}
                activeTab={activeTab}
                selectedSince={selectedSince}
                isFavorite={isFavorite}
                onToggleFavorite={handleToggleFavorite}
              />
            )}
          </div>
        ) : isSearching ? (
          <div className="flex-1 flex flex-col items-center justify-center py-32">
            <div className="w-10 h-10 rounded-full border-2 border-apple-accent/20 border-t-apple-accent animate-spin mb-4" />
            <p className="text-apple-secondary text-sm font-medium">正在搜索全球开源项目...</p>
          </div>
        ) : (
          <div className="flex-1 flex flex-col items-center justify-center py-32 text-apple-secondary">
            <div className="w-20 h-20 bg-apple-bg rounded-3xl flex items-center justify-center mb-6 text-3xl">
              🔍
            </div>
            <h3 className="text-base font-bold text-apple-text mb-2 tracking-tight">发现开源世界</h3>
            <p className="text-[12px] text-apple-secondary/70 max-w-sm text-center leading-relaxed">
              尝试输入自然语言描述，例如"适合初学者的 Rust 项目"或"高性能 Go Web 框架"。
              {aiRewriteEnabled && " AI 将自动优化您的搜索意图。"}
            </p>
          </div>
        )}
      </div>
    </section>
  );

  return (
    <div className="flex h-screen bg-apple-bg font-sans text-apple-text overflow-hidden">
      <Sidebar
        activeTab={activeTab}
        setActiveTab={setActiveTab}
        onSearchClick={handleSearchClick}
      />

      <main className="flex-1 flex overflow-hidden bg-white">
        {activeTab === "settings" ? (
          <SettingsPanel
            modelConfigs={modelConfigs}
            activeConfigId={activeConfigId}
            onSetActive={handleSetActiveConfig}
            onDelete={handleDeleteConfig}
            onAdd={() => {
              setEditingConfig({
                name: "新配置",
                provider: "OpenAI",
                api_base_url: "https://api.openai.com/v1",
                default_model: "gpt-4o-mini",
                enabled: true
              });
              setIsEditingConfig(true);
            }}
            onEdit={(config: any) => {
              setEditingConfig(config);
              setIsEditingConfig(true);
            }}
          />
        ) : activeTab === "search" ? (
          renderSearchPanel()
        ) : (
          <>
            <RepoList
              repos={activeTab === "trending" ? filteredRepos : favoriteRepos}
              selectedRepo={selectedRepo}
              onSelectRepo={handleSelectRepo}
              selectedTopic={selectedTopic}
              setSelectedTopic={setSelectedTopic}
              selectedSince={selectedSince}
              setSelectedSince={setSelectedSince}
              summarizedUrls={summarizedUrls}
              isLoading={activeTab === "trending" && isLoadingTrending}
              activeTab={activeTab}
              onRefresh={fetchTrending}
            />
            <InsightPanel
              selectedRepo={selectedRepo}
              insight={insight}
              isSummarizing={isSummarizing}
              apiKey={apiKey}
              onSummarize={handleSummarize}
              onSettingsClick={handleSettingsClick}
              deepContextEnabled={deepContextEnabled}
              setDeepContextEnabled={setDeepContextEnabled}
              activeTab={activeTab}
              selectedSince={selectedSince}
              isFavorite={isFavorite}
              onToggleFavorite={handleToggleFavorite}
            />
          </>
        )}
      </main>

      {/* Editing Modal - Keep in main App for now due to state complexity */}
      {isEditingConfig && editingConfig && (
        <ConfigModal
          editingConfig={editingConfig}
          setEditingConfig={setEditingConfig}
          onClose={() => setIsEditingConfig(false)}
          onSave={handleSaveConfig}
          onUpdate={handleUpdateConfig}
          isTestingConnection={isTestingConnection}
          testResult={testResult}
          onTest={handleTestConnection}
        />
      )}
    </div>
  );
}

// Additional UI components for Settings to reduce App body size
const SettingsPanel = memo(({ modelConfigs, activeConfigId, onSetActive, onDelete, onAdd, onEdit }: any) => (
  <section className="flex-1 p-10 overflow-y-auto bg-apple-bg/30 tracking-tight">
    <div className="max-w-4xl mx-auto">
      <div className="flex justify-between items-end mb-10">
        <div>
          <h2 className="text-3xl font-extrabold text-apple-text">设置</h2>
          <p className="text-sm text-apple-secondary mt-1">管理 AI 模型配置及应用偏好</p>
        </div>
        <button onClick={onAdd} className="px-5 py-2 bg-apple-accent text-white text-[13px] font-bold rounded-xl shadow-md hover:bg-blue-700 transition-all flex items-center space-x-2">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
            <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
          </svg>
          <span>添加模型配置</span>
        </button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-12">
        {modelConfigs.length === 0 ? (
          <div className="col-span-full py-20 bg-white/50 border-2 border-dashed border-apple-border rounded-3xl flex flex-col items-center justify-center text-apple-secondary">
            <div className="w-16 h-16 bg-white rounded-2xl flex items-center justify-center mb-4 shadow-sm text-2xl">⚙️</div>
            <p className="font-semibold text-sm">暂无模型配置</p>
            <p className="text-[11px] mt-1 opacity-70">点击上方按钮添加您的第一个 AI 模型</p>
          </div>
        ) : (
          modelConfigs.map((config: any) => (
            <div key={config.id} className={`group relative p-6 bg-white rounded-3xl border-2 transition-all ${activeConfigId === config.id ? "border-apple-accent shadow-lg ring-4 ring-apple-accent/5" : "border-apple-border/30 hover:border-apple-accent/30 hover:shadow-md"}`}>
              {activeConfigId === config.id && <div className="absolute top-4 right-4 bg-apple-accent text-white px-2 py-0.5 rounded-full text-[9px] font-black uppercase tracking-widest">Active</div>}
              <div className="flex items-start space-x-4">
                <div className="w-12 h-12 bg-apple-bg rounded-2xl flex items-center justify-center text-xl shadow-inner group-hover:bg-white transition-colors">{typeof config.provider === 'string' ? (config.provider === 'OpenAI' ? '🧠' : config.provider === 'DeepSeek' ? '🐋' : config.provider === 'Anthropic' ? '🎭' : config.provider === 'Google' ? '🌏' : '☁️') : '🛠️'}</div>
                <div className="flex-1 min-w-0">
                  <h3 className="text-base font-bold text-apple-text truncate">{config.name}</h3>
                  <div className="flex items-center space-x-2 mt-1">
                    <span className="text-[10px] font-bold text-apple-accent bg-apple-accent/5 px-2 py-0.5 rounded-md uppercase tracking-tight">{typeof config.provider === 'string' ? config.provider : `Custom (${config.provider.Custom})`}</span>
                  </div>
                </div>
              </div>
              <div className="mt-6 pt-4 border-t border-apple-border/20 space-y-2">
                <div className="flex justify-between text-[11px]"><span className="text-apple-secondary font-medium">模型:</span><span className="text-apple-text font-mono font-bold truncate ml-4">{config.default_model}</span></div>
                <div className="flex justify-between text-[11px]"><span className="text-apple-secondary font-medium">API URL:</span><span className="text-apple-secondary truncate ml-4" title={config.api_base_url}>{config.api_base_url}</span></div>
              </div>
              <div className="mt-8 flex items-center justify-between">
                <div className="flex space-x-2">
                  <button onClick={() => onEdit(config)} className="p-2 text-apple-secondary hover:text-apple-text hover:bg-apple-bg rounded-xl transition-all"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" /><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" /></svg></button>
                  <button onClick={() => onDelete(config.id)} className="p-2 text-apple-secondary hover:text-red-500 hover:bg-red-50 rounded-xl transition-all"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="3 6 5 6 21 6" /><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" /><line x1="10" y1="11" x2="10" y2="17" /><line x1="14" y1="11" x2="14" y2="17" /></svg></button>
                </div>
                <button onClick={() => onSetActive(config.id)} disabled={activeConfigId === config.id} className={`px-4 py-1.5 rounded-xl text-[11px] font-bold transition-all ${activeConfigId === config.id ? "bg-apple-accent/10 text-apple-accent cursor-default" : "bg-apple-bg text-apple-secondary hover:text-apple-text hover:shadow-sm"}`}>{activeConfigId === config.id ? "正在使用" : "激活此配置"}</button>
              </div>
            </div>
          ))
        )}
      </div>

      <div className="pt-8 border-t border-apple-border">
        <h3 className="text-xs font-bold mb-4 text-apple-secondary uppercase tracking-[0.2em] opacity-60">关于应用</h3>
        <div className="bg-white p-6 rounded-3xl border border-apple-border/30">
          <div className="flex items-center justify-between">
            <div><p className="text-sm font-bold text-apple-text">GitHub Capture</p><p className="text-[11px] text-apple-secondary mt-0.5">版本 0.2.0 • 实验性多模型支持</p></div>
            <div className="text-[11px] text-apple-secondary leading-relaxed text-right opacity-80 italic">"探索开源世界，AI 助您一臂之力。"</div>
          </div>
        </div>
      </div>
    </div>
  </section>
));

const ConfigModal = memo(({ editingConfig, setEditingConfig, onClose, onSave, onUpdate, isTestingConnection, testResult, onTest }: any) => (
  <div className="fixed inset-0 z-50 flex items-center justify-center p-6 bg-black/30 backdrop-blur-sm animate-in fade-in duration-200" onClick={onClose}>
    <div className="bg-white w-full max-w-lg rounded-[2.5rem] shadow-2xl border border-apple-border overflow-hidden animate-in zoom-in-95 duration-200" onClick={e => e.stopPropagation()}>
      <div className="p-8 border-b border-apple-border/30 flex justify-between items-center">
        <div><h3 className="text-xl font-extrabold text-apple-text">{editingConfig.id ? "编辑配置" : "添加新配置"}</h3><p className="text-[11px] text-apple-secondary mt-0.5">配置完成后可立即测试连接可用性</p></div>
        <button onClick={onClose} className="w-8 h-8 rounded-full flex items-center justify-center hover:bg-apple-bg transition-colors text-apple-secondary"><svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" /></svg></button>
      </div>
      <div className="p-8 space-y-5 overflow-y-auto max-h-[70vh]">
        <div className="space-y-1.5"><label className="text-[12px] font-bold text-apple-text flex items-center"><span className="w-1 h-3 bg-apple-accent rounded-full mr-2"></span>配置名称</label><input value={editingConfig.name || ""} onChange={e => setEditingConfig((prev: any) => ({ ...prev, name: e.target.value }))} placeholder="例如: DeepSeek-V3" className="w-full px-4 py-3 bg-apple-bg/50 border border-apple-border rounded-2xl outline-none focus:ring-4 focus:ring-apple-accent/10 focus:border-apple-accent/40 text-sm transition-all" /></div>
        <div className="space-y-1.5"><label className="text-[12px] font-bold text-apple-text flex items-center"><span className="w-1 h-3 bg-apple-accent rounded-full mr-2"></span>提供商</label><div className="grid grid-cols-2 gap-2">{["OpenAI", "DeepSeek", "Anthropic", "Google", "AzureOpenAI"].map(p => (<button key={p} onClick={() => setEditingConfig((prev: any) => ({ ...prev, provider: p }))} className={`px-4 py-2.5 rounded-xl border text-[11px] font-bold transition-all ${editingConfig.provider === p ? "bg-apple-accent text-white border-apple-accent" : "bg-apple-bg border-apple-border/50 text-apple-secondary hover:border-apple-accent/30"}`}>{p}</button>))}</div></div>
        <div className="space-y-1.5"><label className="text-[12px] font-bold text-apple-text flex items-center"><span className="w-1 h-3 bg-apple-accent rounded-full mr-2"></span>API 接口地址</label><input value={editingConfig.api_base_url || ""} onChange={e => setEditingConfig((prev: any) => ({ ...prev, api_base_url: e.target.value }))} placeholder="https://api.openai.com/v1" className="w-full px-4 py-3 bg-apple-bg/50 border border-apple-border rounded-2xl outline-none text-sm font-mono" /></div>
        <div className="space-y-1.5"><label className="text-[12px] font-bold text-apple-text flex items-center"><span className="w-1 h-3 bg-apple-accent rounded-full mr-2"></span>默认模型</label><input value={editingConfig.default_model || ""} onChange={e => setEditingConfig((prev: any) => ({ ...prev, default_model: e.target.value }))} placeholder="gpt-4o-mini" className="w-full px-4 py-3 bg-apple-bg/50 border border-apple-border rounded-2xl outline-none text-sm font-mono" /></div>
        <div className="space-y-1.5"><label className="text-[12px] font-bold text-apple-text flex items-center"><span className="w-1 h-3 bg-apple-accent rounded-full mr-2"></span>API Key</label><input type="password" value={editingConfig.api_key || ""} onChange={e => setEditingConfig((prev: any) => ({ ...prev, api_key: e.target.value }))} placeholder="sk-..." className="w-full px-4 py-3 bg-apple-bg/50 border border-apple-border rounded-2xl outline-none text-sm font-mono" /></div>
        {testResult && <div className={`p-4 rounded-2xl text-[11px] font-bold flex items-center ${testResult.success ? "bg-green-50 text-green-600 border border-green-100" : "bg-red-50 text-red-600 border border-red-100"}`}><span className="mr-2 text-sm">{testResult.success ? "✅" : "❌"}</span>{testResult.message}</div>}
      </div>
      <div className="p-8 bg-apple-bg/30 border-t border-apple-border/30 flex justify-between">
        <button onClick={() => onTest(editingConfig.id || "temp")} disabled={isTestingConnection || !editingConfig.api_key} className="px-6 py-2.5 bg-white border border-apple-border/50 text-apple-text text-[12px] font-bold rounded-xl hover:bg-apple-bg transition-all flex items-center space-x-2 disabled:opacity-50 tracking-tight">{isTestingConnection ? <><div className="w-3 h-3 border-2 border-apple-accent/20 border-t-apple-accent rounded-full animate-spin" /><span>测试中...</span></> : <span>测试连接</span>}</button>
        <button onClick={() => editingConfig.id ? onUpdate(editingConfig.id, editingConfig) : onSave(editingConfig as ModelConfig)} className="px-10 py-2.5 bg-apple-accent text-white text-[12px] font-bold rounded-xl hover:bg-blue-700 shadow-md active:scale-95 transition-all">保存配置</button>
      </div>
    </div>
  </div>
));

declare global {
  interface Window {
    __TAURI__?: any;
  }
}

export default App;

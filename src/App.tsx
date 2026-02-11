import { useState, useEffect, useRef, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Channel } from "@tauri-apps/api/core";
import { load } from "@tauri-apps/plugin-store";

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
}

type StreamPayload =
  | { type: "Token", data: string }
  | { type: "Error", data: string }
  | { type: "Done", data: null };

const TOPICS = ["All", "AI / LLM", "Web / App", "Tools / CLI", "Systems / OS", "Mobile", "General"];
const SINCE_OPTIONS = [
  { label: "Today", value: "daily" },
  { label: "Week", value: "weekly" },
  { label: "Month", value: "monthly" }
];

function App() {
  const [activeTab, setActiveTab] = useState("trending");
  const [selectedTopic, setSelectedTopic] = useState("All");
  const [selectedSince, setSelectedSince] = useState("daily");
  const [repos, setRepos] = useState<TrendingRepo[]>([]);
  const [selectedRepo, setSelectedRepo] = useState<TrendingRepo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [insight, setInsight] = useState("");
  const [isSummarizing, setIsSummarizing] = useState(false);

  // Search State
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<TrendingRepo[]>([]);
  const [isSearching, setIsSearching] = useState(false);

  const insightRef = useRef("");

  useEffect(() => {
    initStore();

    // Global shortcut listener (⌘K / Ctrl+K)
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setIsSearchOpen(prev => !prev);
      }
      if (e.key === "Escape") {
        setIsSearchOpen(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  useEffect(() => {
    if (activeTab === "trending") {
      fetchTrending();
    }
  }, [activeTab, selectedSince]);

  useEffect(() => {
    if (selectedRepo && apiKey) {
      handleSummarize(selectedRepo);
    } else {
      setInsight("");
    }
  }, [selectedRepo]);

  const initStore = async () => {
    try {
      const store = await load("settings.json", { autoSave: true });
      const savedKey = await store.get<string>("openai_api_key");
      if (savedKey) setApiKey(savedKey);
    } catch (e) {
      console.error("Store init failed:", e);
    }
  };

  const saveApiKey = async (val: string) => {
    setApiKey(val);
    try {
      const store = await load("settings.json", { autoSave: true });
      await store.set("openai_api_key", val);
    } catch (e) {
      console.error("Save API key failed:", e);
    }
  };

  const fetchTrending = async () => {
    setLoading(true);
    setError(null);
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
      setLoading(false);
    }
  };

  const handleSearch = async () => {
    if (!searchQuery.trim()) return;
    setIsSearching(true);
    try {
      const result: TrendingRepo[] = await invoke("smart_search", {
        query: searchQuery,
        apiKey
      });
      setSearchResults(result);
      if (result.length > 0) {
        setActiveTab("search");
        setIsSearchOpen(false);
        setRepos(result); // Switch trending list to search results
        setSelectedRepo(result[0]);
      }
    } catch (e) {
      console.error("Search failed:", e);
    } finally {
      setIsSearching(false);
    }
  };

  const filteredRepos = useMemo(() => {
    if (selectedTopic === "All") return repos;
    return repos.filter(r => r.topic === selectedTopic || r.topic === "Search Result");
  }, [repos, selectedTopic]);

  const handleSummarize = async (repo: TrendingRepo) => {
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
          language: repo.language
        },
        apiKey,
        onEvent
      });
    } catch (error) {
      console.error("Summarize failed:", error);
      setIsSummarizing(false);
    }
  };

  return (
    <div className="flex h-screen bg-apple-bg select-none font-sans text-apple-text overflow-hidden">
      {/* Search Modal */}
      {isSearchOpen && (
        <div className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh] px-4 backdrop-blur-sm bg-black/10 animate-in fade-in duration-200">
          <div className="w-full max-w-2xl bg-white border border-apple-border rounded-2xl shadow-2xl overflow-hidden animate-in slide-in-from-top-4 duration-300">
            <div className="relative p-6">
              <div className="absolute left-10 top-1/2 -translate-y-1/2 text-apple-accent">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                  <circle cx="11" cy="11" r="8" />
                  <path d="m21 21-4.3-4.3" />
                </svg>
              </div>
              <input
                autoFocus
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleSearch()}
                placeholder="寻找适合学习的 Rust 项目、AI 意图搜索..."
                className="w-full pl-12 pr-4 py-4 text-lg bg-apple-bg/50 border border-apple-border/50 rounded-xl outline-none focus:ring-2 focus:ring-apple-accent/20 font-medium placeholder:text-apple-secondary/50"
              />
            </div>
            <div className="px-6 pb-6 pt-2 border-t border-apple-border/30 bg-apple-bg/5 flex justify-between items-center">
              <div className="flex items-center space-x-4 text-[10px] text-apple-secondary font-bold uppercase tracking-widest">
                <span className="flex items-center"><button className="bg-white border shadow-sm px-1.5 py-0.5 rounded mr-1.5 font-sans">Enter</button> 智能检索</span>
                <span className="flex items-center"><button className="bg-white border shadow-sm px-1.5 py-0.5 rounded mr-1.5 font-sans">Esc</button> 关闭窗口</span>
              </div>
              {isSearching && (
                <div className="flex items-center space-x-2 text-apple-accent">
                  <div className="w-3 h-3 border-2 border-apple-accent/20 border-t-apple-accent rounded-full animate-spin"></div>
                  <span className="text-[10px] font-black uppercase tracking-tighter">AI Processing Intent...</span>
                </div>
              )}
            </div>
          </div>
          <div className="absolute inset-0 -z-10" onClick={() => setIsSearchOpen(false)}></div>
        </div>
      )}

      {/* Sidebar */}
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
            onClick={() => setIsSearchOpen(true)}
            className={`w-full text-left rounded-md flex items-center px-4 py-2 transition-colors ${activeTab === "search" ? "bg-black/5 text-apple-text shadow-sm" : "text-apple-secondary hover:bg-black/5"}`}
          >
            <span className="text-sm font-medium flex-1">智能搜索</span>
            <span className="text-[9px] bg-black/5 px-1 rounded opacity-50">⌘K</span>
          </button>
          <button
            onClick={() => setActiveTab("library")}
            className={`w-full text-left rounded-md flex items-center px-4 py-2 transition-colors ${activeTab === "library" ? "bg-black/5 text-apple-text shadow-sm" : "text-apple-secondary hover:bg-black/5"}`}
          >
            <span className="text-sm font-medium">本地库</span>
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

      {/* Main Content */}
      <main className="flex-1 flex overflow-hidden bg-white">
        {activeTab === "settings" ? (
          <section className="flex-1 p-20 overflow-y-auto">
            <div className="max-w-xl mx-auto">
              <h2 className="text-3xl font-semibold tracking-tight mb-10 text-apple-text">设置</h2>

              <div className="space-y-8">
                <div>
                  <label className="block text-sm font-medium text-apple-text mb-2">OpenAI API Key</label>
                  <input
                    type="password"
                    value={apiKey}
                    onChange={(e) => saveApiKey(e.target.value)}
                    placeholder="sk-..."
                    className="w-full px-4 py-2 bg-apple-bg border border-apple-border rounded-lg outline-none focus:ring-2 focus:ring-apple-accent/20 transition-all text-sm text-apple-text"
                  />
                  <p className="mt-2 text-xs text-apple-secondary">API Key 将加密存储在本地 Store 中。</p>
                </div>

                <div className="pt-8 border-t border-apple-border">
                  <h3 className="text-sm font-semibold mb-4 text-apple-secondary uppercase tracking-widest">关于</h3>
                  <p className="text-sm text-apple-secondary leading-relaxed">
                    GitHub Capture v0.1.0<br />
                    大模型驱动的 GitHub 探索工具
                  </p>
                </div>
              </div>
            </div>
          </section>
        ) : (
          <>
            {/* List Pane */}
            <section className="w-[420px] border-r border-apple-border flex flex-col overflow-hidden">
              <header className="p-5 bg-white/90 backdrop-blur-md sticky top-0 z-10 border-b border-apple-border/30">
                <div className="flex justify-between items-center mb-5">
                  <div className="flex items-center space-x-3 bg-black/5 p-1 rounded-full">
                    {activeTab === "trending" ? SINCE_OPTIONS.map(opt => (
                      <button
                        key={opt.value}
                        onClick={() => setSelectedSince(opt.value)}
                        className={`px-3 py-1 rounded-full text-[10px] font-bold transition-all ${selectedSince === opt.value ? "bg-white text-apple-text shadow-sm" : "text-apple-secondary hover:text-apple-text"}`}
                      >
                        {opt.label}
                      </button>
                    )) : (
                      <div className="px-3 py-1 text-[10px] font-black text-apple-accent uppercase flex items-center">
                        <span className="w-1.5 h-1.5 bg-apple-accent rounded-full mr-2 animate-pulse"></span>
                        Search Results
                      </div>
                    )}
                  </div>

                  <button
                    onClick={() => activeTab === "trending" ? fetchTrending() : setIsSearchOpen(true)}
                    disabled={loading || isSearching}
                    className="relative overflow-hidden flex items-center justify-center min-w-[80px] h-7 bg-apple-accent text-white rounded-full text-[10px] font-extrabold shadow-sm active:scale-95 disabled:bg-apple-accent/50 transition-all font-sans"
                  >
                    <div className={`flex items-center space-x-1.5 px-3 transition-transform duration-300 ${loading || isSearching ? "-translate-y-8" : "translate-y-0"}`}>
                      {activeTab === "trending" ? (
                        <>
                          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="4" strokeLinecap="round" strokeLinejoin="round">
                            <path d="M21 12a9 9 0 1 1-6.219-8.56" />
                            <path d="M22 10V3h-7" />
                          </svg>
                          <span>REFRESH</span>
                        </>
                      ) : (
                        <span>RE-SEARCH</span>
                      )}
                    </div>
                    <div className={`absolute flex items-center space-x-1 transition-transform duration-300 ${loading || isSearching ? "translate-y-0" : "translate-y-8"}`}>
                      <svg className="animate-spin" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="4" strokeLinecap="round" strokeLinejoin="round">
                        <path d="M21 12a9 9 0 1 1-6.219-8.56" />
                      </svg>
                      <span>SYNCING</span>
                    </div>
                  </button>
                </div>

                {/* Topic Tabs */}
                <div className="flex space-x-2 overflow-x-auto pb-1 no-scrollbar selection-none">
                  {TOPICS.map(topic => (
                    <button
                      key={topic}
                      onClick={() => setSelectedTopic(topic)}
                      className={`whitespace-nowrap px-3.5 py-1.5 rounded-full text-[10px] font-bold transition-all border ${selectedTopic === topic ? "bg-apple-text text-white border-apple-text" : "bg-white text-apple-secondary border-apple-border hover:border-apple-accent/50"}`}
                    >
                      {topic}
                    </button>
                  ))}
                </div>
              </header>

              <div className="flex-1 overflow-y-auto divide-y divide-apple-border/20 bg-apple-bg/5 scrollbar-hide">
                {(loading || isSearching) && repos.length === 0 && (
                  <div className="p-24 text-center">
                    <div className="flex justify-center mb-4">
                      <div className="w-8 h-8 rounded-full border-2 border-apple-accent/20 border-t-apple-accent animate-spin"></div>
                    </div>
                    <p className="text-apple-secondary text-[11px] font-bold tracking-widest animate-pulse font-sans">
                      正在通过智能意图分析检索全球项目...
                    </p>
                  </div>
                )}

                {filteredRepos.map((repo) => (
                  <div
                    key={repo.url}
                    onClick={() => setSelectedRepo(repo)}
                    className={`px-6 py-6 cursor-pointer transition-all border-l-2 ${selectedRepo?.url === repo.url ? "bg-white border-apple-accent shadow-sm" : "border-transparent hover:bg-white/50"}`}
                  >
                    <div className="flex items-center space-x-2 mb-2">
                      <span className={`px-1.5 py-0.5 rounded text-[9px] font-extrabold uppercase tracking-tighter ${repo.topic === "Search Result" ? "bg-apple-accent text-white" : "bg-apple-accent/10 text-apple-accent"}`}>
                        {repo.topic}
                      </span>
                    </div>
                    <div className="font-bold text-sm text-apple-text tracking-tight leading-tight">
                      {repo.author} / {repo.name}
                    </div>
                    <div className="text-[11px] text-apple-secondary line-clamp-2 mt-2 leading-relaxed opacity-80">{repo.description || "No description provided."}</div>
                    <div className="flex items-center space-x-5 mt-4 text-[10px] text-apple-secondary font-bold font-sans">
                      <div className="flex items-center">
                        <span className="w-1.5 h-1.5 rounded-full bg-apple-accent mr-1.5"></span>
                        {repo.language}
                      </div>
                      <div className="flex items-center">
                        <span className="mr-1 opacity-60">★</span>
                        {repo.stars}
                      </div>
                      <div className="text-apple-accent">
                        {repo.stars_today ? `+${repo.stars_today.split(" ")[0]} 🚀` : "Found ✨"}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </section>

            {/* Detail/AI Insight Pane */}
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
                        </div>
                      </div>
                      <div className="flex space-x-3 mt-2">
                        <button className="p-2.5 bg-white border border-apple-border rounded-xl hover:bg-apple-bg transition-all shadow-sm">
                          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                            <path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z" />
                          </svg>
                        </button>
                        <a
                          href={selectedRepo.url}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="px-6 py-2.5 bg-black text-white rounded-xl text-xs font-bold hover:shadow-xl hover:bg-black/80 transition-all flex items-center space-x-2 font-sans"
                        >
                          <span>GitHub</span>
                          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
                            <polyline points="15 3 21 3 21 9" />
                            <line x1="10" y1="14" x2="21" y2="3" />
                          </svg>
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
                            <span className="flex space-x-0.5">
                              <span className="w-1 h-1 bg-apple-accent rounded-full animate-bounce"></span>
                              <span className="w-1 h-1 bg-apple-accent rounded-full animate-bounce [animation-delay:0.2s]"></span>
                              <span className="w-1 h-1 bg-apple-accent rounded-full animate-bounce [animation-delay:0.4s]"></span>
                            </span>
                          </span>
                        )}
                      </div>

                      <article>
                        {insight ? (
                          <div className="whitespace-pre-wrap text-[15px] leading-relaxed text-apple-text animate-in fade-in slide-in-from-bottom-3 duration-700 bg-apple-bg/5 p-8 rounded-3xl border border-apple-border/40 selection:bg-apple-accent/20">
                            {insight}
                          </div>
                        ) : (
                          <div className="space-y-8">
                            {!apiKey ? (
                              <div className="flex flex-col items-center justify-center py-32 bg-apple-bg/20 rounded-3xl border border-dashed border-apple-border">
                                <div className="w-16 h-16 bg-white rounded-2xl flex items-center justify-center mb-6 shadow-md text-3xl transform -rotate-12">🔑</div>
                                <p className="text-apple-text text-sm font-bold tracking-tight">配置 API Key 开启技术洞察</p>
                                <p className="text-apple-secondary text-xs mt-2 opacity-70">点击下方按钮前往设置中心</p>
                                <button onClick={() => setActiveTab("settings")} className="mt-6 px-6 py-2.5 bg-apple-accent text-white text-[11px] font-extrabold rounded-full hover:shadow-lg transition-all active:scale-95 font-sans">前往设置 →</button>
                              </div>
                            ) : (
                              <div className="space-y-5 px-4">
                                <div className="h-4 bg-apple-bg rounded-full w-full animate-pulse"></div>
                                <div className="h-4 bg-apple-bg rounded-full w-[90%] animate-pulse [animation-delay:0.2s]"></div>
                                <div className="h-4 bg-apple-bg rounded-full w-[95%] animate-pulse [animation-delay:0.4s]"></div>
                                <div className="h-4 bg-apple-bg rounded-full w-[85%] animate-pulse [animation-delay:0.6s]"></div>
                              </div>
                            )}
                          </div>
                        )}
                      </article>

                      <div className="mt-16 pt-12 border-t border-apple-border/50">
                        <h4 className="text-[10px] font-extrabold uppercase tracking-[0.2em] text-apple-secondary mb-6 opacity-60 font-sans">项目简述与背景</h4>
                        <p className="text-base text-apple-secondary leading-relaxed font-medium bg-apple-bg/5 p-8 rounded-2xl border border-apple-border/20 shadow-inner italic">
                          "{selectedRepo.description || "该项目暂无详细描述"}"
                        </p>
                      </div>
                    </div>
                  </div>
                </>
              ) : (
                <div className="flex-1 flex flex-col items-center justify-center text-apple-secondary space-y-8 bg-apple-bg/10">
                  <div className="relative group">
                    <div className="w-24 h-24 bg-white rounded-[2rem] flex items-center justify-center text-4xl text-apple-text shadow-xl border border-apple-border transform transition-transform group-hover:scale-105 duration-300">⚡️</div>
                    <div className="absolute -top-3 -right-3 w-10 h-10 bg-apple-accent rounded-full flex items-center justify-center text-white text-xs font-black shadow-2xl border-4 border-white animate-bounce">NEW</div>
                  </div>
                  <div className="text-center px-10 max-w-sm">
                    <h3 className="text-base font-black text-apple-text mb-2 tracking-tight">探索 GitHub 趋势</h3>
                    <p className="text-[11px] text-apple-secondary font-medium leading-relaxed opacity-70">点击左侧列表中的项目，我们将结合大模型为您深度剖析其技术架构、核心价值与应用场景。</p>
                  </div>
                </div>
              )}
            </section>
          </>
        )}
      </main>
    </div>
  );
}

declare global {
  interface Window {
    __TAURI__?: any;
  }
}

export default App;

# GitHub Capture - 你的 AI 驱动开源项目深度洞察工具

GitHub Capture 是一款基于桌面端的开源项目探索与分析工具。它将自然语言搜索与 AI 洞察相结合，帮助开发者、技术爱好者和产品经理从浩如烟海的 GitHub 项目中精准打捞最有价值的技术资产。

![GitHub Capture](/path/to/screenshot.png) *(替换为您生成的应用截图)*

## 🚀 核心功能

- **🤖 智能语义搜索**：不再受限于关键词。通过 AI 改写，使用自然语言（如“适合初学者的 Rust AI 框架”）即可精准匹配最相关的仓库。
- **🔥 聚合趋势探索**：实时抓取 GitHub Trending，支持按语言、时间范围（今日、本周、本月）快速筛选热门项目。
- **💡 AI 深度洞察**：一键生成项目总结、技术架构分析、核心痛点解读及快速上手建议。
- **📂 个人项目库**：便捷收藏感兴趣的项目，建立属于你的技术情报库。
- **✨ 极致 UI/UX**：基于现代视觉语言设计，支持分栏预览，提供流畅的桌面端交互体验。

## 🛠️ 技术栈

- **Frontend**: React 19 + TypeScript + Tailwind CSS
- **Backend**: Rust + Tauri v2 (高性能、安全、跨平台)
- **Database**: SQLite (本地存储)
- **AI Engine**: 集成 LLM (支持 OpenAI, Claude, Gemini, DeepSeek 等)

## 📦 开发者设置

### 前置条件

- [Node.js](https://nodejs.org/) (建议 v18+)
- [Rust](https://www.rust-lang.org/) (最新稳定版)
- [pnpm](https://pnpm.io/) 或 npm

### 本地运行

1. **克隆仓库**:
   ```bash
   git clone https://github.com/your-username/githubCapture.git
   cd githubCapture
   ```

2. **安装前端依赖**:
   ```bash
   npm install
   ```

3. **启动开发环境**:
   ```bash
   npm run tauri dev
   ```

### 生产打包

生成 macOS 本地安装包:
```bash
npm run tauri build
```

## 📖 使用指南

1. **配置 AI**: 在设置界面填入您的 LLM API Key（支持主流大模型服务商）。
![img_1.png](img_1.png)
2. **浏览趋势**: 切换到“趋势”页，选择语言和时间范围，查看当前最火的开源项目。
![img.png](img.png)
2. **搜索**: 在“智能搜索”页输入您的技术需求。
![alt text](image.png)
3. **分析**: 选中项目，点击“AI 总结”获取深度技术洞察。
4. **管理**: 点击“星星”图标将项目加入收藏库。

## 🤝 贡献指南

我们非常欢迎社区的贡献！无论是修复 Bug、改进 UI，还是增加新的 AI 分析维度，请随时提交 Pull Request。

1. Fork 本仓库
2. 创建您的特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交您的更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启一个 Pull Request

## 📄 开源协议

本项目采用 **MIT** 协议。详情请参阅 [LICENSE](LICENSE) 文件。

---

*由 Antigravity AI 助力构建。*

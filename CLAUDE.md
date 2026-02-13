# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

GitHub Capture is a Tauri desktop application built with React (TypeScript) and Rust. It fetches GitHub trending repositories, provides AI‑powered summaries, and offers intelligent search with natural‑language query rewriting. The app uses SQLite for local storage and integrates with the GitHub API and OpenAI API.

## Development Commands

### Frontend (npm)
- `npm run dev` – Start Vite dev server (port 1420)
- `npm run build` – Type‑check and build frontend assets to `dist/`
- `npm run preview` – Preview the production build
- `npm run tauri` – Run Tauri CLI (e.g., `npm run tauri dev` to launch the app)

### Backend (Cargo)
- `cd src‑tauri && cargo test` – Run Rust unit tests (defined in `src/*.rs`)
- `cd src‑tauri && cargo build` – Build Rust library for the current target
- `cargo run` is not used directly; the app is launched via `tauri dev`

### Tauri CLI
The `tauri` binary is installed locally; use `npm run tauri` (e.g., `npm run tauri dev`). Common commands:
- `tauri dev` – Start the full application (frontend + Rust backend)
- `tauri build` – Create production bundles for the target platform

## Architecture

The application follows a typical Tauri pattern:
- **Frontend**: React single‑page app using Tauri’s `invoke` API to call Rust commands.
- **Backend**: Rust library (`tauri_app_lib`) exposing commands via `#[tauri::command]`.
- **Data persistence**: SQLite database managed by `tauri‑plugin‑sql` with migrations.
- **External APIs**: GitHub (trending scraping, REST search) and OpenAI (chat completions).

### Key Rust Modules (`src‑tauri/src/`)
- `lib.rs` – Registers plugins and commands.
- `trending.rs` – Scrapes GitHub trending pages, classifies repos by topic, sorts by stars.
- `search.rs` – Provides `search_github` (GitHub API) and `ai_rewrite_query` (OpenAI‑powered query optimization).
- `ai.rs` – `summarize_repo` streams an AI‑generated technical analysis of a repository.
- `db.rs` – Defines SQLite migrations for `repos`, `insights`, and `search_history` tables.

### Frontend Structure (`src/`)
- `App.tsx` – Main component with three tabs: Trending, Search, Settings.
- `index.css` – Tailwind CSS with custom “apple” design tokens.
- `main.tsx` – Entry point.
- `vite‑env.d.ts` – TypeScript definitions.

### Styling
- Tailwind CSS with a custom theme (`tailwind.config.js`) that defines `apple` colors and fonts.
- PostCSS with `autoprefixer`.

### Database
SQLite file `github_capture.db` is created automatically in Tauri's app data directory (platform‑dependent). Migrations are defined in `db.rs` (version 1). The schema includes:
- `repos` – Cached trending/search results.
- `insights` – AI‑generated summaries keyed by repo URL.
- `search_history` – Log of user queries.

### AI Integration
- OpenAI’s chat completions API (`gpt‑4o‑mini` for summaries, `gpt‑3.5‑turbo` for query rewriting).
- API key is stored securely in the Tauri Store (`settings.json`).
- Streaming is used for summaries to provide real‑time output.

### Tauri Configuration
- `src‑tauri/tauri.conf.json` – App identifier `com.githubcapture.app`, dev URL `http://localhost:1420`.
- `src‑tauri/Cargo.toml` – Dependencies include `tauri`, `tauri‑plugin‑*`, `reqwest`, `scraper`, `tokio`, etc.
- Frontend assets are served from `../dist` after build.

## Testing
- Rust unit tests are placed in `#[cfg(test)]` modules (e.g., `trending.rs` contains a test for `fetch_trending`).
- Run all tests with `cargo test` from the `src‑tauri` directory.
- No frontend test suite is currently configured.

## Notes for Developers
- The trending scraper depends on GitHub’s HTML structure; changes may break parsing.
- AI features require a valid OpenAI API key entered in the Settings tab.
- The app uses `reqwest‑eventsource` for streaming OpenAI responses.
- Tauri’s `Channel` is used to send streamed tokens to the frontend.
- All GitHub API requests include a custom `User‑Agent` header.
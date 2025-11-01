# Repository Guidelines

## 项目结构与模块组织
- `app/`：Next.js App Router 页面与布局（示例：`app/page.tsx`, `app/layout.tsx`）。
- `lib/`：通用工具与类型（如 `lib/utils.ts` 的 `cn`）。
- `public/`：静态资源。
- `src-tauri/`：Tauri 桌面端（`src/`, `Cargo.toml`, `tauri.conf.json`, `icons/`）。
- 根配置：`next.config.ts`, `tsconfig.json`, `eslint.config.mjs`, `components.json`。

## 构建、测试与本地开发
- 安装依赖：`pnpm i`
- Web 开发：`pnpm dev`（http://localhost:3000）
- 桌面开发：`pnpm tauri:dev`（内置 `WEBKIT_DISABLE_DMABUF_RENDERER=1` 以提升 Linux/Wayland/NVIDIA 兼容性）
- Web 构建：`pnpm build`（静态导出到 `out/`，见 `next.config.ts` 的 `output: "export"`）
- 桌面打包：`pnpm tauri:build`
- 代码检查：`pnpm lint`

## 代码风格与命名约定
- TypeScript 严格模式；优先函数式 React 组件。
- 缩进 2 空格；文件名 kebab-case，组件名 PascalCase。
- 路径别名：`@/*`；合并类名使用 `cn(...)`（见 `lib/utils.ts`）。
- Tailwind CSS v4；全局样式在 `app/globals.css`；避免内联 style。
- ESLint：`eslint-config-next`（core-web-vitals + typescript）；提交前确保无报错。

## 测试指南
- 当前未集成测试框架；建议引入 Vitest + @testing-library/react。
- 测试命名：`*.test.ts(x)`；位置：与模块同级或 `__tests__/`。
- 引入后请在 `package.json` 增加 `test` 脚本，并覆盖核心路径与关键逻辑。

## Commit 与 Pull Request 规范
- 遵循 Conventional Commits：
  - 示例：`feat(linux): add Wayland+NVIDIA compatibility`、`refactor: convert template`、`chore: initialize tauri`。
- PR 需包含：变更摘要、动机/影响、关联 Issue、UI/桌面改动截图或录屏、手动验证步骤。
- 合并前运行：`pnpm lint` 与必要的构建命令（`pnpm build`/`pnpm tauri:build`）。

## 环境与安全提示
- 需要 Rust ≥ 1.77 与 Tauri 依赖；Node 与 pnpm 版本以锁文件为准。
- 权限与图标见 `src-tauri/tauri.conf.json` 与 `src-tauri/icons/`；仅申请必要权限，注意日志级别在发布版收敛。


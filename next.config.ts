import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // 启用静态导出（Next 13+/App Router 推荐方式）
  output: "export",
  // 使用 next/image 时，静态导出需关闭内置图片优化
  images: {
    unoptimized: true,
  },
  // 如需在不带后缀的路径上稳定访问静态文件，可考虑开启
  // trailingSlash: true,
};

export default nextConfig;

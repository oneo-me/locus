import { type RouteConfig, index, route } from "@react-router/dev/routes";

export default [
  index("routes/home.tsx"),
  route("zh-CN", "routes/home.zh-CN.tsx"),
  route("zh-TW", "routes/home.zh-TW.tsx"),
  route("ja", "routes/home.ja.tsx"),
  route("ko", "routes/home.ko.tsx"),
] satisfies RouteConfig;

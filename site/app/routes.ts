import { type RouteConfig, index, route } from "@react-router/dev/routes";

export default [
  index("routes/home.tsx"),
  route("zh-CN", "routes/home.zh-CN.tsx"),
] satisfies RouteConfig;

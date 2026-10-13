import type { Route } from "./+types/home.zh-CN";
import { Landing } from "~/components/landing";
import { content } from "~/content";

export function meta({}: Route.MetaArgs) {
  return [
    { title: content["zh-CN"].title },
    { name: "description", content: content["zh-CN"].description },
  ];
}

export default function HomeZhCN() {
  return <Landing locale="zh-CN" />;
}

import type { Route } from "./+types/home.zh-TW";
import { Landing } from "~/components/landing";
import { content } from "~/content";

export function meta({}: Route.MetaArgs) {
  return [
    { title: content["zh-TW"].title },
    { name: "description", content: content["zh-TW"].description },
  ];
}

export default function HomeZhTW() {
  return <Landing locale="zh-TW" />;
}

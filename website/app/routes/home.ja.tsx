import type { Route } from "./+types/home.ja";
import { Landing } from "~/components/landing";
import { content } from "~/content";

export function meta({}: Route.MetaArgs) {
  return [
    { title: content.ja.title },
    { name: "description", content: content.ja.description },
  ];
}

export default function HomeJa() {
  return <Landing locale="ja" />;
}

import type { Route } from "./+types/home.ko";
import { Landing } from "~/components/landing";
import { content } from "~/content";

export function meta({}: Route.MetaArgs) {
  return [
    { title: content.ko.title },
    { name: "description", content: content.ko.description },
  ];
}

export default function HomeKo() {
  return <Landing locale="ko" />;
}

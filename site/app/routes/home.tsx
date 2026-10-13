import type { Route } from "./+types/home";
import { Landing } from "~/components/landing";
import { content } from "~/content";

export function meta({}: Route.MetaArgs) {
  return [
    { title: content.en.title },
    { name: "description", content: content.en.description },
  ];
}

export default function Home() {
  return <Landing locale="en" />;
}

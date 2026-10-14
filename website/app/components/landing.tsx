import { ArrowUpRight, Download } from "lucide-react";
import { Link } from "react-router";

import { GitHubIcon } from "~/components/github-icon";
import { LanguageMenu } from "~/components/language-menu";
import { ThemeMenu } from "~/components/theme-menu";
import { Badge } from "~/components/ui/badge";
import { Button } from "~/components/ui/button";
import {
  Card,
  CardDescription,
  CardHeader,
  CardTitle,
} from "~/components/ui/card";
import { Separator } from "~/components/ui/separator";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "~/components/ui/tooltip";
import {
  content,
  downloadUrl,
  localePaths,
  releasesUrl,
  repositoryUrl,
  type Locale,
} from "~/content";

const iconUrl = `${import.meta.env.BASE_URL}icon.png`;

export function Landing({ locale }: { locale: Locale }) {
  const text = content[locale];

  return (
    <div className="flex min-h-svh flex-col">
      <header className="mx-auto flex w-full max-w-5xl flex-wrap items-center justify-between gap-3 px-6 py-5">
        <Link to={localePaths[locale]} className="flex items-center gap-2 font-semibold">
          <img src={iconUrl} alt="" className="size-7" />
          Locus
        </Link>
        <nav className="flex flex-wrap items-center justify-end gap-1">
          <LanguageMenu locale={locale} text={text} />
          <ThemeMenu text={text} />
          <Tooltip>
            <TooltipTrigger
              render={
                <Button
                  variant="ghost"
                  size="icon-sm"
                  nativeButton={false}
                  aria-label={text.source}
                  render={
                    <a href={repositoryUrl} target="_blank" rel="noreferrer" />
                  }
                />
              }
            >
              <GitHubIcon />
            </TooltipTrigger>
            <TooltipContent>{text.source}</TooltipContent>
          </Tooltip>
        </nav>
      </header>

      <main className="flex-1">
        <section className="mx-auto flex max-w-3xl flex-col items-center px-6 pt-16 pb-20 text-center sm:pt-24">
          <img src={iconUrl} alt="" className="mb-8 size-24 sm:size-28" />
          <Badge variant="secondary">{text.badge}</Badge>
          <h1 className="mt-5 text-4xl font-semibold tracking-tight text-balance sm:text-5xl">
            {text.heading}
          </h1>
          <p className="mt-5 max-w-2xl text-lg text-pretty text-muted-foreground">
            {text.lead}
          </p>
          <div className="mt-9 flex flex-col gap-3 sm:flex-row">
            <Button
              size="lg"
              className="h-11 px-5 text-base"
              nativeButton={false}
              render={<a href={downloadUrl} />}
            >
              <Download data-icon="inline-start" />
              {text.download}
            </Button>
            <Button
              size="lg"
              variant="outline"
              className="h-11 px-5 text-base"
              nativeButton={false}
              render={<a href={repositoryUrl} />}
            >
              {text.source}
              <ArrowUpRight data-icon="inline-end" />
            </Button>
          </div>
          <p className="mt-4 text-sm text-muted-foreground">{text.platform}</p>
        </section>

        <section className="mx-auto max-w-5xl px-6 pb-24">
          <h2 className="mb-6 text-center text-2xl font-semibold tracking-tight">
            {text.featuresHeading}
          </h2>
          <div className="grid gap-4 sm:grid-cols-2">
            {text.features.map((feature) => (
              <Card key={feature.title}>
                <CardHeader>
                  <CardTitle>{feature.title}</CardTitle>
                  <CardDescription>{feature.body}</CardDescription>
                </CardHeader>
              </Card>
            ))}
          </div>
        </section>
      </main>

      <footer className="mx-auto w-full max-w-5xl px-6 pb-10">
        <Separator className="mb-6" />
        <div className="flex flex-col gap-2 text-sm text-muted-foreground sm:flex-row sm:justify-between">
          <span>© ONEO · {text.license}</span>
          <a href={releasesUrl} className="hover:text-foreground">
            {text.allReleases}
          </a>
        </div>
      </footer>
    </div>
  );
}

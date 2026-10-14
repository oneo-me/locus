import { Check, ChevronDown, Languages } from "lucide-react";
import { Link } from "react-router";

import { Button } from "~/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "~/components/ui/dropdown-menu";
import {
  content,
  locales,
  localePaths,
  type Content,
  type Locale,
} from "~/content";

export function LanguageMenu({
  locale,
  text,
}: {
  locale: Locale;
  text: Content;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            variant="ghost"
            size="sm"
            aria-label={`${text.languageLabel}: ${text.languageName}`}
          />
        }
      >
        <Languages data-icon="inline-start" />
        {text.languageName}
        <ChevronDown data-icon="inline-end" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-40">
        <DropdownMenuGroup>
          {locales.map((other) => (
            <DropdownMenuItem
              key={other}
              render={<Link to={localePaths[other]} hrefLang={other} />}
              aria-current={other === locale ? "page" : undefined}
            >
              {content[other].languageName}
              {other === locale && <Check className="ml-auto" />}
            </DropdownMenuItem>
          ))}
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

import { Monitor, Moon, Sun } from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "~/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "~/components/ui/dropdown-menu";
import type { Content } from "~/content";
import {
  applyTheme,
  getStoredTheme,
  storeTheme,
  type Theme,
} from "~/lib/theme";

export function ThemeMenu({ text }: { text: Content }) {
  const [theme, setTheme] = useState<Theme>("system");

  useEffect(() => {
    const stored = getStoredTheme();
    applyTheme(stored);
    setTheme(stored);
  }, []);

  useEffect(() => {
    if (theme !== "system") return;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => applyTheme("system");
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, [theme]);

  function selectTheme(next: Theme) {
    storeTheme(next);
    applyTheme(next);
    setTheme(next);
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={text.themeLabel}
          />
        }
      >
        {theme === "light" ? <Sun /> : theme === "dark" ? <Moon /> : <Monitor />}
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-40">
        <DropdownMenuGroup>
          <DropdownMenuRadioGroup
            value={theme}
            onValueChange={(value) => selectTheme(value as Theme)}
          >
            <DropdownMenuRadioItem value="light" closeOnClick>
              <Sun />
              {text.themeLight}
            </DropdownMenuRadioItem>
            <DropdownMenuRadioItem value="dark" closeOnClick>
              <Moon />
              {text.themeDark}
            </DropdownMenuRadioItem>
            <DropdownMenuRadioItem value="system" closeOnClick>
              <Monitor />
              {text.themeSystem}
            </DropdownMenuRadioItem>
          </DropdownMenuRadioGroup>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

import type { AgentSessionSelectConfig } from '@locus/agent/contracts/conversation';
import type { AgentProviderId } from '@locus/agent/contracts/provider';
import {
  CheckIcon,
  ChevronDownIcon,
  SearchIcon,
  SparklesIcon,
  StarIcon,
} from 'lucide-react';
import { useState } from 'react';
import { Button } from '@locus/ui/components/button';
import { Input } from '@locus/ui/components/input';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@locus/ui/components/popover';
import { cn } from '@locus/ui/utils';

const FAVORITE_MODELS_KEY = 'locus.agent.favorite-models';

export function AgentModelPicker({
  disabled,
  onChange,
  option,
  providerId,
}: {
  disabled: boolean;
  onChange: (value: string) => void;
  option: AgentSessionSelectConfig;
  providerId: AgentProviderId;
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [favoritesOnly, setFavoritesOnly] = useState(false);
  const [favorites, setFavorites] = useState<string[]>(readFavorites);
  const selected = option.options.find(
    (item) => item.value === option.currentValue,
  );
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const visible = option.options.filter((item) => {
    if (
      favoritesOnly &&
      !favorites.includes(favoriteKey(providerId, item.value))
    ) {
      return false;
    }
    return (
      !normalizedQuery ||
      `${item.name} ${item.value}`.toLocaleLowerCase().includes(normalizedQuery)
    );
  });

  const toggleFavorite = (modelId: string) => {
    const key = favoriteKey(providerId, modelId);
    setFavorites((current) => {
      const next = current.includes(key)
        ? current.filter((item) => item !== key)
        : [...current, key];
      writeFavorites(next);
      return next;
    });
  };

  return (
    <Popover
      onOpenChange={(next) => {
        setOpen(next);
        if (next) {
          setQuery('');
          setFavoritesOnly(false);
        }
      }}
      open={open}
    >
      <PopoverTrigger
        render={
          <Button
            aria-label={option.name}
            className="h-7 max-w-64 min-w-0 justify-between gap-1.5 rounded-[min(var(--radius-md),10px)] border-0 bg-transparent py-2 pr-2 pl-2.5 text-sm shadow-none"
            disabled={disabled}
            size="nav"
            variant="nav"
          >
            <SparklesIcon aria-hidden="true" className="size-4" />
            <span className="min-w-0 flex-1 truncate text-left">
              {selected?.name ?? option.currentValue}
            </span>
            <ChevronDownIcon
              aria-hidden="true"
              className="size-4 text-muted-foreground"
            />
          </Button>
        }
      />
      <PopoverContent
        align="start"
        className="h-[390px] w-[520px] max-w-[calc(100vw-2rem)] gap-0 overflow-hidden p-0"
        side="top"
      >
        <div className="flex shrink-0 items-center gap-1 border-b p-2">
          <Button
            aria-pressed={!favoritesOnly}
            onClick={() => setFavoritesOnly(false)}
            size="sm"
            variant={!favoritesOnly ? 'secondary' : 'ghost'}
          >
            全部
          </Button>
          <Button
            aria-pressed={favoritesOnly}
            onClick={() => setFavoritesOnly(true)}
            size="sm"
            variant={favoritesOnly ? 'secondary' : 'ghost'}
          >
            <StarIcon aria-hidden="true" data-icon="inline-start" />
            星标
          </Button>
        </div>
        <div className="shrink-0 border-b p-2">
          <div className="relative">
            <SearchIcon className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              aria-label="搜索模型"
              autoFocus
              className="pl-8"
              onChange={(event) => setQuery(event.target.value)}
              placeholder="搜索模型"
              value={query}
            />
          </div>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto p-2">
          {visible.length === 0 ? (
            <div className="grid h-full place-items-center text-muted-foreground">
              {favoritesOnly ? '还没有星标模型' : '没有匹配的模型'}
            </div>
          ) : (
            <div className="flex flex-col gap-0.5" role="listbox">
              {visible.map((model) => {
                const favorite = favorites.includes(
                  favoriteKey(providerId, model.value),
                );
                const isSelected = model.value === option.currentValue;
                return (
                  <div
                    aria-selected={isSelected}
                    className={cn(
                      'flex min-h-14 cursor-default items-center gap-2 rounded-lg px-3 py-2 outline-none hover:bg-accent focus-visible:bg-accent',
                      isSelected && 'bg-accent',
                    )}
                    key={model.value}
                    onClick={() => {
                      onChange(model.value);
                      setOpen(false);
                    }}
                    onKeyDown={(event) => {
                      if (event.key !== 'Enter' && event.key !== ' ') return;
                      event.preventDefault();
                      onChange(model.value);
                      setOpen(false);
                    }}
                    role="option"
                    tabIndex={0}
                  >
                    <span className="min-w-0 flex-1">
                      <span className="block break-words leading-5">
                        {model.name}
                      </span>
                      {model.value !== model.name && (
                        <span className="mt-0.5 block break-all text-xs text-muted-foreground">
                          {model.value}
                        </span>
                      )}
                    </span>
                    {isSelected && (
                      <CheckIcon className="size-4 shrink-0 text-muted-foreground" />
                    )}
                    <button
                      aria-label={favorite ? '取消星标' : '添加星标'}
                      className="grid size-8 shrink-0 place-items-center rounded-md text-muted-foreground hover:bg-background/70 hover:text-foreground"
                      onClick={(event) => {
                        event.stopPropagation();
                        toggleFavorite(model.value);
                      }}
                      type="button"
                    >
                      <StarIcon
                        aria-hidden="true"
                        className={cn(
                          'size-4',
                          favorite && 'fill-amber-500 text-amber-500',
                        )}
                      />
                    </button>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}

function favoriteKey(providerId: AgentProviderId, modelId: string): string {
  return `${providerId}:${modelId}`;
}

function readFavorites(): string[] {
  if (typeof window === 'undefined') return [];
  try {
    const value = JSON.parse(
      window.localStorage.getItem(FAVORITE_MODELS_KEY) ?? '[]',
    );
    return Array.isArray(value)
      ? value.filter((item): item is string => typeof item === 'string')
      : [];
  } catch {
    return [];
  }
}

function writeFavorites(favorites: string[]): void {
  try {
    window.localStorage.setItem(FAVORITE_MODELS_KEY, JSON.stringify(favorites));
  } catch {
    return;
  }
}

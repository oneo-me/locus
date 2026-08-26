import type { AgentSessionSelectConfig } from '@locus/agent/contracts/conversation';
import type { AgentProviderId } from '@locus/agent/contracts/provider';
import { SearchIcon, StarIcon } from 'lucide-react';
import { useState } from 'react';
import { Button } from '@locus/ui/components/button';
import { Empty, EmptyHeader, EmptyTitle } from '@locus/ui/components/empty';
import { Input } from '@locus/ui/components/input';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@locus/ui/components/popover';
import { Separator } from '@locus/ui/components/separator';
import { cn } from '@locus/ui/utils';
import { AgentProviderIcon } from '~/features/agent/provider-icon';

const FAVORITE_MODELS_KEY = 'locus.agent.favorite-models';

interface AgentModelSource {
  id: AgentProviderId;
  name: string;
  options: AgentSessionSelectConfig['options'];
}

export function AgentModelPicker({
  disabled,
  onChange,
  selectedModelId,
  selectedProviderId,
  sources,
}: {
  disabled: boolean;
  onChange: (providerId: AgentProviderId, modelId: string) => void;
  selectedModelId: string;
  selectedProviderId: AgentProviderId;
  sources: AgentModelSource[];
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [sourceFilter, setSourceFilter] = useState<
    'favorites' | AgentProviderId
  >(selectedProviderId);
  const [favorites, setFavorites] = useState<string[]>(readFavorites);
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const hasAvailableFavorites = sources.some((source) =>
    source.options.some((model) =>
      favorites.includes(favoriteKey(source.id, model.value)),
    ),
  );
  const selectedSource = sources.find(
    (source) => source.id === selectedProviderId,
  );
  const selected = selectedSource?.options.find(
    (model) => model.value === selectedModelId,
  );
  const visibleSources = normalizedQuery
    ? sources
    : sources.filter(
        (source) => sourceFilter === 'favorites' || source.id === sourceFilter,
      );
  const visible = visibleSources.flatMap((source) =>
    source.options
      .filter((model) => {
        const favorite = favorites.includes(
          favoriteKey(source.id, model.value),
        );
        if (!normalizedQuery && sourceFilter === 'favorites' && !favorite) {
          return false;
        }
        if (!normalizedQuery) return true;
        return `${model.name} ${model.value} ${modelProviderName(model.value, source.id)} ${source.name}`
          .toLocaleLowerCase()
          .includes(normalizedQuery);
      })
      .map((model) => ({ model, source })),
  );

  const toggleFavorite = (providerId: AgentProviderId, modelId: string) => {
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
          setSourceFilter(
            hasAvailableFavorites ? 'favorites' : selectedProviderId,
          );
        }
      }}
      open={open}
    >
      <PopoverTrigger
        render={
          <Button
            aria-label="选择模型"
            className="max-w-64 min-w-0 justify-start"
            disabled={disabled}
            size="nav"
            variant="ghost"
          >
            <AgentProviderIcon providerId={selectedProviderId} />
            <span className="min-w-0 truncate">
              {selected
                ? modelDisplayName(selected.name)
                : modelDisplayName(selectedModelId)}
            </span>
          </Button>
        }
      />
      <PopoverContent
        align="start"
        aria-label="选择模型"
        className="h-[390px] w-[460px] max-w-[calc(100vw-2rem)] flex-row gap-0 overflow-hidden p-0"
        side="top"
      >
        <nav
          aria-label="Agent"
          className="flex w-12 shrink-0 flex-col items-center gap-1 overflow-y-auto border-r bg-background p-1.5"
        >
          <Button
            aria-label="收藏模型"
            aria-pressed={sourceFilter === 'favorites' && !normalizedQuery}
            onClick={() => {
              setSourceFilter('favorites');
              setQuery('');
            }}
            size="icon"
            title="收藏模型"
            variant={
              sourceFilter === 'favorites' && !normalizedQuery
                ? 'selected'
                : 'ghost'
            }
          >
            <StarIcon aria-hidden="true" />
          </Button>
          <Separator className="my-1 w-8" />
          {sources.map((source) => (
            <Button
              aria-label={source.name}
              aria-pressed={sourceFilter === source.id && !normalizedQuery}
              key={source.id}
              onClick={() => {
                setSourceFilter(source.id);
                setQuery('');
              }}
              size="icon"
              title={source.name}
              variant={
                sourceFilter === source.id && !normalizedQuery
                  ? 'selected'
                  : 'ghost'
              }
            >
              <AgentProviderIcon providerId={source.id} />
            </Button>
          ))}
        </nav>
        <div className="flex min-w-0 flex-1 flex-col">
          <div className="shrink-0 p-2.5 pb-2">
            <div className="relative">
              <SearchIcon className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                aria-label="搜索模型"
                autoFocus
                className="pl-8"
                onChange={(event) => setQuery(event.target.value)}
                placeholder="搜索模型…"
                value={query}
              />
            </div>
          </div>
          <div className="min-h-0 flex-1 overflow-y-auto p-2">
            {visible.length === 0 ? (
              <Empty className="h-full p-0">
                <EmptyHeader>
                  <EmptyTitle>
                    {normalizedQuery
                      ? '没有匹配的模型'
                      : sourceFilter === 'favorites'
                        ? '还没有收藏模型'
                        : 'Agent 没有提供模型'}
                  </EmptyTitle>
                </EmptyHeader>
              </Empty>
            ) : (
              <div
                aria-label="模型"
                className="flex flex-col gap-0.5"
                role="listbox"
              >
                {visible.map(({ model, source }) => {
                  const favorite = favorites.includes(
                    favoriteKey(source.id, model.value),
                  );
                  const isSelected =
                    source.id === selectedProviderId &&
                    model.value === selectedModelId;
                  return (
                    <div
                      aria-selected={isSelected}
                      className={cn(
                        'flex h-[58px] cursor-default items-center gap-2.5 rounded-lg border border-transparent px-3 outline-none hover:bg-accent focus-visible:border-ring focus-visible:bg-accent',
                        isSelected && 'bg-accent',
                      )}
                      key={`${source.id}:${model.value}`}
                      onClick={() => {
                        onChange(source.id, model.value);
                        setOpen(false);
                      }}
                      onKeyDown={(event) => {
                        if (event.key !== 'Enter' && event.key !== ' ') return;
                        event.preventDefault();
                        onChange(source.id, model.value);
                        setOpen(false);
                      }}
                      role="option"
                      tabIndex={0}
                    >
                      <span className="min-w-0 flex-1">
                        <span className="block truncate font-medium">
                          {modelDisplayName(model.name)}
                        </span>
                        <span className="mt-1 flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
                          <AgentProviderIcon
                            className="size-3"
                            providerId={source.id}
                          />
                          <span className="truncate">
                            {modelProviderName(model.value, source.id)} ·{' '}
                            {source.name}
                          </span>
                        </span>
                      </span>
                      <Button
                        aria-label={favorite ? '取消收藏' : '添加收藏'}
                        aria-pressed={favorite}
                        onClick={(event) => {
                          event.stopPropagation();
                          toggleFavorite(source.id, model.value);
                        }}
                        onKeyDown={(event) => event.stopPropagation()}
                        size="icon-sm"
                        variant="ghost"
                      >
                        <StarIcon
                          aria-hidden="true"
                          className={cn(
                            favorite && 'fill-yellow-400 text-yellow-500',
                          )}
                        />
                      </Button>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      </PopoverContent>
    </Popover>
  );
}

function favoriteKey(providerId: AgentProviderId, modelId: string): string {
  return `${providerId}:${modelId}`;
}

function modelDisplayName(name: string): string {
  const separator = name.indexOf('/');
  return separator >= 0 ? name.slice(separator + 1).trim() || name : name;
}

function modelProviderName(
  modelId: string,
  agentProviderId: AgentProviderId,
): string {
  const namespace = modelId.includes('/') ? modelId.split('/')[0] : null;
  if (namespace) {
    const known: Record<string, string> = {
      anthropic: 'Anthropic',
      'amazon-bedrock': 'Amazon Bedrock',
      azure: 'Azure OpenAI',
      'azure-openai': 'Azure OpenAI',
      deepseek: 'DeepSeek',
      google: 'Google',
      'google-vertex': 'Google Vertex',
      groq: 'Groq',
      mistral: 'Mistral',
      moonshot: 'Moonshot',
      openai: 'OpenAI',
      opencode: 'OpenCode',
      openrouter: 'OpenRouter',
      vertexai: 'Google Vertex',
      xai: 'xAI',
    };
    return known[namespace.toLocaleLowerCase()] ?? displayName(namespace);
  }
  const defaults: Record<AgentProviderId, string> = {
    amp: 'Amp',
    claude: 'Anthropic',
    codex: 'OpenAI',
    cursor: 'Cursor',
    deepseek: 'DeepSeek',
    fx: 'Fx',
    grok: 'xAI',
    kimi: 'Moonshot',
    ohmypi: 'Oh My Pi',
    opencode: 'OpenCode',
    pi: 'Pi',
  };
  return defaults[agentProviderId];
}

function displayName(value: string): string {
  return value
    .split(/[-_]/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toLocaleUpperCase() + part.slice(1))
    .join(' ');
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

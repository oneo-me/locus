import FluentIconArrowUpload from '@oneo/fluentui-icons-like-react/FluentIconArrowUpload';
import FluentIconDelete from '@oneo/fluentui-icons-like-react/FluentIconDelete';
import { useRef, useState } from 'react';
import { Button } from '~/components/ui/button';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '~/components/ui/popover';
import { cn } from '~/lib/utils';
import { DocumentIcon, documentEmojiGroups } from './icons';

export interface DocumentMetadataPatch {
  icon?: string | null;
}

export function DocumentIconPicker({
  className,
  disabled,
  icon,
  iconClassName,
  onChange,
  imageSrc,
  onImageRemove,
  onImageUpload,
}: {
  className?: string;
  disabled?: boolean;
  icon: string | null;
  iconClassName?: string;
  onChange: (value: string | null) => void;
  imageSrc?: string | null;
  onImageRemove: () => void;
  onImageUpload: (file: File) => void;
}) {
  const [open, setOpen] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const select = (next: string | null) => {
    setOpen(false);
    if (next !== icon) onChange(next);
  };

  return (
    <Popover
      onOpenChange={(nextOpen) => {
        setOpen(nextOpen);
      }}
      open={open}
    >
      <PopoverTrigger
        aria-label="文档图标"
        className={cn(
          'grid size-7 place-items-center rounded-[min(var(--radius-md),12px)] border border-input bg-transparent text-sm transition-colors outline-none select-none hover:bg-muted focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50',
          className,
        )}
        disabled={disabled}
        title={imageSrc ? '自定义图片图标' : (icon ?? undefined)}
      >
        <DocumentIcon
          className={cn('size-4', iconClassName)}
          icon={icon}
          imageSrc={imageSrc}
        />
      </PopoverTrigger>
      <PopoverContent align="start" className="w-72 gap-2 p-2">
        <div className="flex items-center justify-between gap-1.5">
          <p className="text-muted-foreground px-1 text-xs">选择图标</p>
          <Button
            aria-label="移除图标"
            disabled={!icon && !imageSrc}
            onClick={() => {
              if (imageSrc) {
                setOpen(false);
                onImageRemove();
              } else {
                select(null);
              }
            }}
            size="icon-xs"
            title="移除图标"
            type="button"
            variant="ghost"
          >
            <FluentIconDelete aria-hidden="true" title={null} />
          </Button>
        </div>
        <div className="flex flex-col gap-1.5">
          <input
            accept="image/*"
            className="sr-only"
            disabled={disabled}
            onChange={(event) => {
              const file = event.currentTarget.files?.[0];
              event.currentTarget.value = '';
              if (!file) return;
              setOpen(false);
              onImageUpload(file);
            }}
            ref={fileInputRef}
            type="file"
          />
          <Button
            className="w-full"
            disabled={disabled}
            onClick={() => fileInputRef.current?.click()}
            size="sm"
            type="button"
            variant="outline"
          >
            <FluentIconArrowUpload
              aria-hidden="true"
              data-icon="inline-start"
              title={null}
            />
            {imageSrc ? '替换图片' : '上传图片'}
          </Button>
          <p className="px-1 text-[11px] leading-4 text-muted-foreground">
            图片会居中裁切为 64×64 PNG。
          </p>
        </div>
        <div className="max-h-64 overflow-y-auto outline-none">
          <div className="flex flex-col gap-3">
            {documentEmojiGroups.map((group) => (
              <section
                aria-label={group.label}
                className="flex flex-col gap-1.5"
                key={group.id}
              >
                <h3 className="text-muted-foreground px-1 text-[11px] font-medium tracking-wide">
                  {group.label}
                </h3>
                <div className="grid grid-cols-8 gap-0.5">
                  {group.emojis.map((emoji) => (
                    <button
                      aria-label={emoji}
                      aria-pressed={icon === emoji}
                      className={cn(
                        'grid size-7 place-items-center rounded-md text-base leading-none transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:outline-none',
                        icon === emoji && 'bg-accent',
                      )}
                      key={`${group.id}-${emoji}`}
                      onClick={() => select(emoji)}
                      title={emoji}
                      type="button"
                    >
                      {emoji}
                    </button>
                  ))}
                </div>
              </section>
            ))}
          </div>
        </div>
      </PopoverContent>
    </Popover>
  );
}

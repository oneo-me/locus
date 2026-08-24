import { useRef } from 'react';
import { Button } from '@locus/ui/components/button';
import { cn } from '@locus/ui/utils';
import { DocumentIcon } from './icons';

export function DocumentIconUpload({
  className,
  disabled,
  icon,
  iconClassName,
  imageSrc,
  onImageUpload,
}: {
  className?: string;
  disabled?: boolean;
  icon: string | null;
  iconClassName?: string;
  imageSrc?: string | null;
  onImageUpload: (file: File) => void;
}) {
  const fileInputRef = useRef<HTMLInputElement>(null);

  return (
    <>
      <input
        accept="image/*"
        disabled={disabled}
        hidden
        onChange={(event) => {
          const file = event.currentTarget.files?.[0];
          event.currentTarget.value = '';
          if (file) onImageUpload(file);
        }}
        ref={fileInputRef}
        type="file"
      />
      <Button
        aria-label={imageSrc ? '替换文档图标图片' : '上传文档图标图片'}
        className={cn('shrink-0 shadow-none leading-none', className)}
        disabled={disabled}
        onClick={() => fileInputRef.current?.click()}
        size="icon-sm"
        title={imageSrc ? '替换文档图标图片' : '上传文档图标图片'}
        type="button"
        variant="ghost"
      >
        <DocumentIcon
          className={cn('size-4', iconClassName)}
          icon={icon}
          imageSrc={imageSrc}
        />
      </Button>
    </>
  );
}

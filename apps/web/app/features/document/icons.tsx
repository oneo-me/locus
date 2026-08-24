import { useState } from 'react';
import { cn } from '@locus/ui/utils';

const DEFAULT_DOCUMENT_ICON = '📄';

export function DocumentIcon({
  className,
  icon,
  imageSrc,
}: {
  className?: string;
  icon: string | null;
  imageSrc?: string | null;
}) {
  const value = icon ?? DEFAULT_DOCUMENT_ICON;

  return (
    <span
      aria-hidden="true"
      className={cn(
        'inline-flex items-center justify-center overflow-hidden leading-none',
        className,
      )}
    >
      {imageSrc ? (
        <DocumentIconImage fallback={value} key={imageSrc} src={imageSrc} />
      ) : (
        value
      )}
    </span>
  );
}

function DocumentIconImage({
  fallback,
  src,
}: {
  fallback: string;
  src: string;
}) {
  const [failed, setFailed] = useState(false);
  return failed ? (
    fallback
  ) : (
    <img
      alt=""
      className="size-full object-cover"
      decoding="async"
      onError={() => setFailed(true)}
      src={src}
    />
  );
}

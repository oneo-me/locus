import { useId } from 'react';
import {
  ATMOSPHERE_SHAPE_VIEWBOX,
  type AtmosphereBackdrop,
  type AtmosphereShape,
  type AtmosphereShapeInstance,
  type AtmosphereTheme,
  atmosphereArcPath,
  deriveDocumentAtmosphere,
} from './atmosphere';

function BackdropPattern({
  backdrop,
  patternId,
}: {
  backdrop: Exclude<AtmosphereBackdrop, { kind: 'none' }>;
  patternId: string;
}) {
  if (backdrop.kind === 'dots') {
    return (
      <pattern
        height={backdrop.spacing}
        id={patternId}
        patternUnits="userSpaceOnUse"
        width={backdrop.spacing}
        x={backdrop.offsetX}
        y={backdrop.offsetY}
      >
        <circle
          cx={backdrop.spacing / 2}
          cy={backdrop.spacing / 2}
          fill={backdrop.fill}
          fillOpacity={backdrop.opacity}
          r={backdrop.radius}
        />
      </pattern>
    );
  }

  const midpoint = backdrop.spacing / 2;
  let path: string;

  if (backdrop.kind === 'grid') {
    path = `M 0 0 V ${backdrop.spacing} M 0 0 H ${backdrop.spacing}`;
  } else if (backdrop.kind === 'lines') {
    path = `M 0 0 H ${backdrop.spacing}`;
  } else if (backdrop.kind === 'diagonal') {
    path = `M 0 0 L ${backdrop.spacing} ${backdrop.spacing}`;
  } else if (backdrop.kind === 'diamonds') {
    path = `M 0 ${midpoint} L ${midpoint} 0 L ${backdrop.spacing} ${midpoint} L ${midpoint} ${backdrop.spacing} Z`;
  } else {
    path = `M 0 ${midpoint} Q ${backdrop.spacing / 4} ${midpoint - backdrop.amplitude} ${midpoint} ${midpoint} Q ${(backdrop.spacing * 3) / 4} ${midpoint + backdrop.amplitude} ${backdrop.spacing} ${midpoint}`;
  }

  return (
    <pattern
      height={backdrop.spacing}
      id={patternId}
      patternTransform={`rotate(${backdrop.rotate})`}
      patternUnits="userSpaceOnUse"
      width={backdrop.spacing}
      x={backdrop.offsetX}
      y={backdrop.offsetY}
    >
      <path
        d={path}
        fill="none"
        stroke={backdrop.stroke}
        strokeOpacity={backdrop.opacity}
        strokeWidth={backdrop.strokeWidth}
      />
    </pattern>
  );
}

function shapeKey(shape: AtmosphereShape) {
  if (shape.kind === 'disk') {
    return `disk-${shape.cx}-${shape.cy}-${shape.r}`;
  }
  if (shape.kind === 'ring') {
    return `ring-${shape.cx}-${shape.cy}-${shape.r}-${shape.strokeWidth}`;
  }
  if (shape.kind === 'roundedRect') {
    return `rect-${shape.x}-${shape.y}-${shape.width}-${shape.height}-${shape.rotate}`;
  }
  if (shape.kind === 'arc') {
    return `arc-${shape.cx}-${shape.cy}-${shape.r}-${shape.startDeg}-${shape.sweepDeg}`;
  }
  return `triangle-${shape.points}`;
}

function AtmosphereShapeNode({ shape }: { shape: AtmosphereShape }) {
  if (shape.kind === 'disk') {
    return (
      <circle
        cx={shape.cx}
        cy={shape.cy}
        fill={shape.fill}
        fillOpacity={shape.opacity}
        r={shape.r}
      />
    );
  }

  if (shape.kind === 'ring') {
    return (
      <circle
        cx={shape.cx}
        cy={shape.cy}
        fill="none"
        r={shape.r}
        stroke={shape.stroke}
        strokeOpacity={shape.opacity}
        strokeWidth={shape.strokeWidth}
      />
    );
  }

  if (shape.kind === 'roundedRect') {
    return (
      <rect
        fill={shape.fill}
        fillOpacity={shape.opacity}
        height={shape.height}
        rx={shape.rx}
        transform={`rotate(${shape.rotate} ${shape.x + shape.width / 2} ${shape.y + shape.height / 2})`}
        width={shape.width}
        x={shape.x}
        y={shape.y}
      />
    );
  }

  if (shape.kind === 'arc') {
    return (
      <path
        d={atmosphereArcPath(shape)}
        fill="none"
        stroke={shape.stroke}
        strokeLinecap="round"
        strokeOpacity={shape.opacity}
        strokeWidth={shape.strokeWidth}
      />
    );
  }

  return (
    <polygon
      fill={shape.fill}
      fillOpacity={shape.opacity}
      points={shape.points}
    />
  );
}

function StaticShape({ instance }: { instance: AtmosphereShapeInstance }) {
  return <AtmosphereShapeNode shape={instance.shape} />;
}

function AtmosphereThemeLayers({
  idPrefix,
  theme,
  title,
}: {
  idPrefix: string;
  theme: AtmosphereTheme;
  title: string;
}) {
  const atmosphere = deriveDocumentAtmosphere(title, theme);
  const fadeId = `doc-atmosphere-fade-${idPrefix}`;
  const backdropMaskId = `doc-atmosphere-backdrop-mask-${idPrefix}`;
  const shapeMaskId = `doc-atmosphere-shape-mask-${idPrefix}`;
  const shapeHorizontalMaskId = `doc-atmosphere-shape-horizontal-mask-${idPrefix}`;
  const patternId = `doc-atmosphere-backdrop-${idPrefix}`;
  const { width, height } = ATMOSPHERE_SHAPE_VIEWBOX;
  return (
    <>
      <div
        className="absolute inset-0"
        style={{
          backgroundImage: `linear-gradient(to bottom, ${atmosphere.gradientColor}, transparent)`,
        }}
      />

      {atmosphere.backdrop.kind !== 'none' ? (
        <svg
          aria-hidden="true"
          className="absolute inset-0 size-full"
          focusable="false"
        >
          <defs>
            <linearGradient id={fadeId} x1="0" x2="0" y1="0" y2="1">
              <stop offset="0%" stopColor="white" stopOpacity="1" />
              <stop offset="70%" stopColor="white" stopOpacity="0.45" />
              <stop offset="100%" stopColor="white" stopOpacity="0" />
            </linearGradient>
            <BackdropPattern
              backdrop={atmosphere.backdrop}
              patternId={patternId}
            />
            <mask
              height="100%"
              id={backdropMaskId}
              maskUnits="userSpaceOnUse"
              width="100%"
              x="0"
              y="0"
            >
              <rect
                fill={`url(#${fadeId})`}
                height="100%"
                width="100%"
                x="0"
                y="0"
              />
            </mask>
          </defs>
          <rect
            fill={`url(#${patternId})`}
            height="124%"
            mask={`url(#${backdropMaskId})`}
            width="116%"
            x="-8%"
            y="-12%"
          />
        </svg>
      ) : null}

      <div className="relative mx-auto h-52 w-[calc(100%-3rem)] max-w-[52rem]">
        <svg
          aria-hidden="true"
          className="absolute inset-0 size-full"
          focusable="false"
          preserveAspectRatio="xMidYMid meet"
          viewBox={`0 0 ${width} ${height}`}
        >
          <defs>
            <linearGradient id={`${fadeId}-shape`} x1="0" x2="0" y1="0" y2="1">
              <stop offset="0%" stopColor="white" stopOpacity="1" />
              <stop offset="75%" stopColor="white" stopOpacity="0.55" />
              <stop offset="100%" stopColor="white" stopOpacity="0" />
            </linearGradient>
            <linearGradient
              id={`${fadeId}-shape-horizontal`}
              x1="0"
              x2="1"
              y1="0"
              y2="0"
            >
              <stop offset="0%" stopColor="white" stopOpacity="0" />
              <stop offset="34%" stopColor="white" stopOpacity="0" />
              <stop offset="52%" stopColor="white" stopOpacity="1" />
              <stop offset="100%" stopColor="white" stopOpacity="1" />
            </linearGradient>
            <mask id={shapeMaskId}>
              <rect
                fill={`url(#${fadeId}-shape)`}
                height={height}
                width={width}
                x="0"
                y="0"
              />
            </mask>
            <mask id={shapeHorizontalMaskId}>
              <rect
                fill={`url(#${fadeId}-shape-horizontal)`}
                height={height}
                width={width}
                x="0"
                y="0"
              />
            </mask>
          </defs>
          <g mask={`url(#${shapeMaskId})`}>
            <g mask={`url(#${shapeHorizontalMaskId})`}>
              {atmosphere.shapes.map((instance) => (
                <StaticShape
                  instance={instance}
                  key={shapeKey(instance.shape)}
                />
              ))}
            </g>
          </g>
        </svg>
      </div>
    </>
  );
}

/**
 * 文档主栏顶部氛围：全宽渐变与静态线网背景 + 文章栏宽内独立图形层。
 * 亮/暗两套同时输出，由 `<html>.dark`（内联脚本首屏已写入）切换可见性，
 * 不读 React `resolvedTheme`，避免刷新首屏色错与 hydration 卡死样式。
 */
export function DocumentHeaderAtmosphere({ title }: { title: string }) {
  const reactId = useId().replace(/:/g, '');

  return (
    <div
      aria-hidden="true"
      className="pointer-events-none absolute inset-x-0 top-0 h-[calc(13rem+60px)] overflow-hidden"
      data-document-header-atmosphere
    >
      <div className="absolute inset-0 dark:hidden">
        <AtmosphereThemeLayers
          idPrefix={`${reactId}-light`}
          theme="light"
          title={title}
        />
      </div>
      <div className="absolute inset-0 hidden dark:block">
        <AtmosphereThemeLayers
          idPrefix={`${reactId}-dark`}
          theme="dark"
          title={title}
        />
      </div>
    </div>
  );
}

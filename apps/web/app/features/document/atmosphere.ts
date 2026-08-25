export type AtmosphereTheme = 'light' | 'dark';

export type AtmosphereBackdrop =
  | { kind: 'none' }
  | {
      kind: 'grid';
      spacing: number;
      rotate: number;
      offsetX: number;
      offsetY: number;
      strokeWidth: number;
      opacity: number;
      stroke: string;
    }
  | {
      kind: 'lines';
      spacing: number;
      rotate: number;
      offsetX: number;
      offsetY: number;
      strokeWidth: number;
      opacity: number;
      stroke: string;
    }
  | {
      kind: 'dots';
      spacing: number;
      offsetX: number;
      offsetY: number;
      radius: number;
      opacity: number;
      fill: string;
    }
  | {
      kind: 'diagonal';
      spacing: number;
      rotate: number;
      offsetX: number;
      offsetY: number;
      strokeWidth: number;
      opacity: number;
      stroke: string;
    }
  | {
      kind: 'diamonds';
      spacing: number;
      rotate: number;
      offsetX: number;
      offsetY: number;
      strokeWidth: number;
      opacity: number;
      stroke: string;
    }
  | {
      kind: 'waves';
      spacing: number;
      amplitude: number;
      rotate: number;
      offsetX: number;
      offsetY: number;
      strokeWidth: number;
      opacity: number;
      stroke: string;
    };

export type AtmosphereShape =
  | {
      kind: 'disk';
      cx: number;
      cy: number;
      r: number;
      opacity: number;
      fill: string;
    }
  | {
      kind: 'ring';
      cx: number;
      cy: number;
      r: number;
      strokeWidth: number;
      opacity: number;
      stroke: string;
    }
  | {
      kind: 'roundedRect';
      x: number;
      y: number;
      width: number;
      height: number;
      rx: number;
      rotate: number;
      opacity: number;
      fill: string;
    }
  | {
      kind: 'arc';
      cx: number;
      cy: number;
      r: number;
      startDeg: number;
      sweepDeg: number;
      strokeWidth: number;
      opacity: number;
      stroke: string;
    }
  | {
      kind: 'triangle';
      points: string;
      opacity: number;
      fill: string;
    };

/** 图形层固定坐标系，配合 SVG viewBox 保持比例，避免随容器拉伸变形。 */
export const ATMOSPHERE_SHAPE_VIEWBOX = {
  width: 360,
  height: 100,
} as const;

export type AtmosphereMotionMode = 'float' | 'spin';

export interface AtmosphereShapeMotion {
  mode: AtmosphereMotionMode;
  duration: number;
  delay: number;
  driftX: number;
  driftY: number;
  swayX: number;
  swayY: number;
  rotateFrom: number;
  rotateMid: number;
  rotateTo: number;
}

export interface AtmosphereShapeInstance {
  shape: AtmosphereShape;
  motion: AtmosphereShapeMotion;
}

export interface AtmosphereBackdropMotion {
  duration: number;
  delay: number;
  driftX: number;
  driftY: number;
  swayX: number;
  swayY: number;
}

export interface DocumentAtmosphere {
  hue: number;
  gradientColor: string;
  backdrop: AtmosphereBackdrop;
  backdropMotion: AtmosphereBackdropMotion | null;
  shapes: AtmosphereShapeInstance[];
}

function hashValue(value: string) {
  let hash = 2166136261;
  for (const character of value) {
    hash ^= character.charCodeAt(0);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

/** 与主哈希混合出稳定的后续样本，避免共用同一低位域。 */
function mix(hash: number, salt: number) {
  let next = hash >>> 0;
  next ^= salt >>> 0;
  next = Math.imul(next ^ (next >>> 16), 2246822507);
  next = Math.imul(next ^ (next >>> 13), 3266489909);
  return (next ^ (next >>> 16)) >>> 0;
}

function unit(hash: number) {
  return (hash >>> 0) / 0x1_0000_0000;
}

function round(value: number, digits = 3) {
  const factor = 10 ** digits;
  return Math.round(value * factor) / factor;
}

function clamp(value: number, min: number, max: number) {
  if (min > max) return (min + max) / 2;
  return Math.min(Math.max(value, min), max);
}

function tone(
  hue: number,
  theme: AtmosphereTheme,
  role: 'fill' | 'stroke',
  salt: number,
) {
  const isDark = theme === 'dark';
  if (role === 'fill') {
    const lightness = isDark ? 52 + (salt % 10) : 56 + (salt % 12);
    const saturation = isDark ? 32 + (salt % 10) : 36 + (salt % 10);
    return `hsl(${hue} ${saturation}% ${lightness}%)`;
  }

  const lightness = isDark ? 68 + (salt % 8) : 40 + (salt % 8);
  const saturation = isDark ? 28 + (salt % 8) : 32 + (salt % 8);
  return `hsl(${hue} ${saturation}% ${lightness}%)`;
}

function deriveBackdrop(
  seed: number,
  hue: number,
  theme: AtmosphereTheme,
): AtmosphereBackdrop {
  const local = mix(seed, 0xbd01);
  const variant = mix(local, 1) % 7;
  const isDark = theme === 'dark';
  const opacity = round(
    (isDark ? 0.055 : 0.038) + unit(mix(local, 2)) * 0.035,
    3,
  );
  const stroke = tone(hue, theme, 'stroke', mix(local, 3) % 16);
  const spacing = round(18 + unit(mix(local, 4)) * 16, 2);
  const rotate = round(-8 + unit(mix(local, 5)) * 16, 2);
  const offsetX = round(unit(mix(local, 6)) * spacing, 2);
  const offsetY = round(unit(mix(local, 7)) * spacing, 2);
  const strokeWidth = round(0.5 + unit(mix(local, 8)) * 0.45, 2);

  if (variant === 0) return { kind: 'none' };

  if (variant === 1) {
    return {
      kind: 'grid',
      spacing,
      rotate,
      offsetX,
      offsetY,
      strokeWidth,
      opacity: round(opacity * 0.72, 3),
      stroke,
    };
  }

  if (variant === 2) {
    return {
      kind: 'lines',
      spacing,
      rotate,
      offsetX,
      offsetY,
      strokeWidth,
      opacity,
      stroke,
    };
  }

  if (variant === 3) {
    return {
      kind: 'dots',
      spacing,
      offsetX,
      offsetY,
      radius: round(0.55 + unit(mix(local, 9)) * 0.55, 2),
      opacity,
      fill: stroke,
    };
  }

  if (variant === 4) {
    return {
      kind: 'diagonal',
      spacing: round(spacing * 1.08, 2),
      rotate: round(rotate + 24, 2),
      offsetX,
      offsetY,
      strokeWidth,
      opacity,
      stroke,
    };
  }

  if (variant === 5) {
    return {
      kind: 'diamonds',
      spacing: round(spacing * 1.16, 2),
      rotate: round(rotate * 0.45, 2),
      offsetX,
      offsetY,
      strokeWidth: round(strokeWidth * 0.9, 2),
      opacity: round(opacity * 0.78, 3),
      stroke,
    };
  }

  const waveSpacing = round(spacing * 1.24, 2);
  return {
    kind: 'waves',
    spacing: waveSpacing,
    amplitude: round(
      clamp(waveSpacing * (0.11 + unit(mix(local, 10)) * 0.05), 2.4, 5.8),
      2,
    ),
    rotate: round(rotate * 0.32, 2),
    offsetX,
    offsetY,
    strokeWidth: round(strokeWidth * 0.92, 2),
    opacity: round(opacity * 0.84, 3),
    stroke,
  };
}

function deriveBackdropMotion(seed: number): AtmosphereBackdropMotion {
  const local = mix(seed, 0xbda7);
  const driftAngle = unit(mix(local, 1)) * Math.PI * 2;
  const driftScale = 13 + unit(mix(local, 2)) * 9;
  const swayAngle = driftAngle + Math.PI * (0.72 + unit(mix(local, 3)) * 0.56);
  const swayScale = driftScale * (0.64 + unit(mix(local, 4)) * 0.22);

  return {
    duration: round(140 + unit(mix(local, 5)) * 80, 2),
    delay: round(unit(mix(local, 6)) * -120, 2),
    driftX: round(Math.cos(driftAngle) * driftScale, 2),
    driftY: round(Math.sin(driftAngle) * driftScale * 0.58, 2),
    swayX: round(Math.cos(swayAngle) * swayScale, 2),
    swayY: round(Math.sin(swayAngle) * swayScale * 0.58, 2),
  };
}

function arcPath(
  cx: number,
  cy: number,
  r: number,
  startDeg: number,
  sweepDeg: number,
) {
  const start = (startDeg * Math.PI) / 180;
  const end = ((startDeg + sweepDeg) * Math.PI) / 180;
  const x1 = round(cx + Math.cos(start) * r, 2);
  const y1 = round(cy + Math.sin(start) * r, 2);
  const x2 = round(cx + Math.cos(end) * r, 2);
  const y2 = round(cy + Math.sin(end) * r, 2);
  const largeArc = Math.abs(sweepDeg) > 180 ? 1 : 0;
  const sweep = sweepDeg >= 0 ? 1 : 0;
  return `M ${x1} ${y1} A ${r} ${r} 0 ${largeArc} ${sweep} ${x2} ${y2}`;
}

function deriveShapeMotion(
  local: number,
  kind: AtmosphereShape['kind'],
  prominence: number,
): AtmosphereShapeMotion {
  const mode: AtmosphereMotionMode = kind === 'arc' ? 'spin' : 'float';
  const normalizedProminence = clamp(prominence, 0.35, 1);
  const activity = 1 - normalizedProminence;
  const driftScale = 16 + activity * 11 + unit(mix(local, 29)) * 4;
  const rotateAmp = 8 + unit(mix(local, 30)) * 12;
  const spinDirection = mix(local, 31) % 2 === 0 ? 1 : -1;
  const driftAngle = unit(mix(local, 34)) * Math.PI * 2;

  if (mode === 'spin') {
    const spinDriftScale = 8 + unit(mix(local, 36)) * 6;
    return {
      mode,
      duration: round(88 + unit(mix(local, 32)) * 42, 2),
      delay: round(unit(mix(local, 33)) * -58, 2),
      driftX: round(Math.cos(driftAngle) * spinDriftScale, 2),
      driftY: round(Math.sin(driftAngle) * spinDriftScale * 0.68, 2),
      swayX: 0,
      swayY: 0,
      rotateFrom: 0,
      rotateMid: 180 * spinDirection,
      rotateTo: 360 * spinDirection,
    };
  }

  const swayAngle = driftAngle + Math.PI * (0.72 + unit(mix(local, 35)) * 0.56);
  const driftX = round(Math.cos(driftAngle) * driftScale, 2);
  const driftY = round(Math.sin(driftAngle) * driftScale * 0.68, 2);
  const swayScale = driftScale * (0.7 + unit(mix(local, 36)) * 0.24);
  const swayX = round(Math.cos(swayAngle) * swayScale, 2);
  const swayY = round(Math.sin(swayAngle) * swayScale * 0.68, 2);

  return {
    mode,
    duration: round(
      56 + normalizedProminence * 12 + unit(mix(local, 32)) * 24,
      2,
    ),
    delay: round(unit(mix(local, 33)) * -44, 2),
    driftX,
    driftY,
    swayX,
    swayY,
    rotateFrom: round(-rotateAmp * 0.35 * spinDirection, 2),
    rotateMid: round(rotateAmp * spinDirection, 2),
    rotateTo: round(-rotateAmp * 0.72 * spinDirection, 2),
  };
}

interface CompositionSlot {
  x: number;
  y: number;
  scale: number;
  jitterX: number;
  jitterY: number;
}

/**
 * 多套槽位版式提供不同的视觉重心；每个槽位仍保留安全尺度与有限扰动，
 * 让结果有明显差异，同时避免随机坐标造成拥挤或残缺图形。
 */
const COMPOSITION_LAYOUTS: ReadonlyArray<readonly CompositionSlot[]> = [
  [
    { x: 286, y: 38, scale: 1, jitterX: 15, jitterY: 9 },
    { x: 214, y: 74, scale: 0.74, jitterX: 13, jitterY: 8 },
    { x: 328, y: 74, scale: 0.55, jitterX: 9, jitterY: 7 },
    { x: 243, y: 19, scale: 0.48, jitterX: 11, jitterY: 6 },
    { x: 170, y: 54, scale: 0.42, jitterX: 8, jitterY: 10 },
    { x: 333, y: 18, scale: 0.38, jitterX: 8, jitterY: 5 },
  ],
  [
    { x: 270, y: 66, scale: 1, jitterX: 16, jitterY: 9 },
    { x: 328, y: 30, scale: 0.72, jitterX: 10, jitterY: 8 },
    { x: 205, y: 27, scale: 0.58, jitterX: 13, jitterY: 8 },
    { x: 238, y: 79, scale: 0.48, jitterX: 12, jitterY: 6 },
    { x: 176, y: 65, scale: 0.4, jitterX: 9, jitterY: 8 },
    { x: 315, y: 82, scale: 0.36, jitterX: 9, jitterY: 5 },
  ],
  [
    { x: 305, y: 50, scale: 1, jitterX: 15, jitterY: 10 },
    { x: 229, y: 24, scale: 0.7, jitterX: 13, jitterY: 8 },
    { x: 210, y: 78, scale: 0.58, jitterX: 12, jitterY: 7 },
    { x: 333, y: 20, scale: 0.48, jitterX: 9, jitterY: 6 },
    { x: 272, y: 82, scale: 0.42, jitterX: 11, jitterY: 6 },
    { x: 172, y: 48, scale: 0.36, jitterX: 8, jitterY: 10 },
  ],
  [
    { x: 246, y: 45, scale: 1, jitterX: 16, jitterY: 10 },
    { x: 321, y: 65, scale: 0.74, jitterX: 11, jitterY: 8 },
    { x: 296, y: 18, scale: 0.58, jitterX: 12, jitterY: 6 },
    { x: 189, y: 74, scale: 0.48, jitterX: 10, jitterY: 7 },
    { x: 343, y: 26, scale: 0.42, jitterX: 7, jitterY: 7 },
    { x: 170, y: 28, scale: 0.36, jitterX: 8, jitterY: 8 },
  ],
  [
    { x: 294, y: 68, scale: 1, jitterX: 16, jitterY: 9 },
    { x: 244, y: 24, scale: 0.74, jitterX: 13, jitterY: 8 },
    { x: 333, y: 45, scale: 0.58, jitterX: 9, jitterY: 9 },
    { x: 204, y: 72, scale: 0.5, jitterX: 11, jitterY: 7 },
    { x: 175, y: 34, scale: 0.42, jitterX: 8, jitterY: 8 },
    { x: 276, y: 84, scale: 0.36, jitterX: 11, jitterY: 5 },
  ],
  [
    { x: 263, y: 31, scale: 1, jitterX: 16, jitterY: 9 },
    { x: 320, y: 75, scale: 0.72, jitterX: 11, jitterY: 7 },
    { x: 208, y: 67, scale: 0.58, jitterX: 12, jitterY: 8 },
    { x: 341, y: 29, scale: 0.48, jitterX: 8, jitterY: 7 },
    { x: 176, y: 52, scale: 0.42, jitterX: 9, jitterY: 10 },
    { x: 250, y: 82, scale: 0.36, jitterX: 11, jitterY: 5 },
  ],
];

const COMPOSITION_FAMILIES: ReadonlyArray<readonly AtmosphereShape['kind'][]> =
  [
    ['disk', 'ring', 'arc', 'disk', 'ring', 'triangle'],
    ['roundedRect', 'arc', 'disk', 'ring', 'roundedRect', 'triangle'],
    ['triangle', 'arc', 'roundedRect', 'ring', 'disk', 'arc'],
    ['ring', 'roundedRect', 'disk', 'triangle', 'arc', 'roundedRect'],
    ['arc', 'disk', 'triangle', 'ring', 'roundedRect', 'disk'],
    ['disk', 'triangle', 'roundedRect', 'arc', 'ring', 'triangle'],
    ['ring', 'arc', 'roundedRect', 'disk', 'triangle', 'ring'],
    ['roundedRect', 'ring', 'triangle', 'disk', 'arc', 'roundedRect'],
    ['triangle', 'disk', 'arc', 'roundedRect', 'ring', 'disk'],
    ['arc', 'roundedRect', 'ring', 'triangle', 'disk', 'arc'],
  ];

function deriveShapes(
  seed: number,
  hue: number,
  theme: AtmosphereTheme,
): AtmosphereShapeInstance[] {
  const localRoot = mix(seed, 0x5a9e);
  const count = 4 + (mix(localRoot, 1) % 3);
  const shapes: AtmosphereShapeInstance[] = [];
  const { width, height } = ATMOSPHERE_SHAPE_VIEWBOX;
  const layout =
    COMPOSITION_LAYOUTS[mix(localRoot, 2) % COMPOSITION_LAYOUTS.length] ??
    COMPOSITION_LAYOUTS[0];
  const family =
    COMPOSITION_FAMILIES[mix(localRoot, 3) % COMPOSITION_FAMILIES.length] ??
    COMPOSITION_FAMILIES[0];

  for (let index = 0; index < count; index += 1) {
    const local = mix(localRoot, 20 + index * 11);
    const slot = layout?.[index] ?? layout?.[0] ?? COMPOSITION_LAYOUTS[0]?.[0];
    if (!slot) continue;
    const kind = family?.[index] ?? 'disk';
    const prominence = clamp(
      slot.scale * (0.88 + unit(mix(local, 1)) * 0.24),
      0.34,
      1.08,
    );
    const motion = deriveShapeMotion(local, kind, prominence);
    const opacity = round(
      (theme === 'dark' ? 0.085 : 0.065) +
        prominence * 0.035 +
        unit(mix(local, 2)) * (theme === 'dark' ? 0.045 : 0.035),
      3,
    );
    const hueShift = [-9, 6, -4, 10, 2, -7][index] ?? 0;
    const shapeHue = clamp(hue + hueShift, 190, 250);
    const fill = tone(shapeHue, theme, 'fill', mix(local, 3) % 20);
    const stroke = tone(shapeHue, theme, 'stroke', mix(local, 4) % 20);
    const rawCx = slot.x + (unit(mix(local, 5)) * 2 - 1) * slot.jitterX;
    const rawCy = slot.y + (unit(mix(local, 6)) * 2 - 1) * slot.jitterY;
    const motionMarginX =
      Math.max(Math.abs(motion.driftX), Math.abs(motion.swayX)) * 0.65;
    const motionMarginY =
      Math.max(Math.abs(motion.driftY), Math.abs(motion.swayY)) * 0.65;
    const centerFor = (extent: number) => ({
      cx: round(
        clamp(
          rawCx,
          132 + extent + motionMarginX,
          width - 4 - extent - motionMarginX,
        ),
        2,
      ),
      cy: round(
        clamp(
          rawCy,
          4 + extent + motionMarginY,
          height - 4 - extent - motionMarginY,
        ),
        2,
      ),
    });

    let shape: AtmosphereShape;

    if (kind === 'disk') {
      const r = round((17 + unit(mix(local, 7)) * 11) * prominence, 2);
      const { cx, cy } = centerFor(r);
      shape = {
        kind,
        cx,
        cy,
        r,
        opacity,
        fill,
      };
    } else if (kind === 'ring') {
      const r = round((17 + unit(mix(local, 7)) * 10) * prominence, 2);
      const strokeWidth = round(
        1.15 + unit(mix(local, 8)) * 1.45 * prominence,
        2,
      );
      const { cx, cy } = centerFor(r + strokeWidth);
      shape = {
        kind,
        cx,
        cy,
        r,
        strokeWidth,
        opacity: round(opacity + 0.018, 3),
        stroke,
      };
    } else if (kind === 'roundedRect') {
      const rectHeight = round((18 + unit(mix(local, 7)) * 8) * prominence, 2);
      const aspectRatio = 1.45 + unit(mix(local, 8)) * 0.55;
      const rectWidth = round(rectHeight * aspectRatio, 2);
      const extent = Math.hypot(rectWidth, rectHeight) / 2;
      const { cx, cy } = centerFor(extent);
      shape = {
        kind,
        x: round(cx - rectWidth / 2, 2),
        y: round(cy - rectHeight / 2, 2),
        width: rectWidth,
        height: rectHeight,
        rx: round(
          Math.min(
            rectHeight * 0.22,
            (4 + unit(mix(local, 9)) * 3) * prominence,
          ),
          2,
        ),
        rotate: round(-14 + unit(mix(local, 10)) * 28, 2),
        opacity,
        fill,
      };
    } else if (kind === 'arc') {
      const r = round((18 + unit(mix(local, 7)) * 10) * prominence, 2);
      const strokeWidth = round(
        1.2 + unit(mix(local, 10)) * 1.8 * prominence,
        2,
      );
      const { cx, cy } = centerFor(r + strokeWidth);
      shape = {
        kind,
        cx,
        cy,
        r,
        startDeg: round(unit(mix(local, 8)) * 360, 1),
        sweepDeg: round(105 + unit(mix(local, 9)) * 105, 1),
        strokeWidth,
        opacity: round(opacity + 0.024, 3),
        stroke,
      };
    } else {
      const size = round((20 + unit(mix(local, 7)) * 11) * prominence, 2);
      const { cx, cy } = centerFor(size);
      const angle = unit(mix(local, 8)) * Math.PI * 2;
      const points = [0, 1, 2]
        .map((corner) => {
          const theta = angle + (Math.PI * 2 * corner) / 3;
          return `${round(cx + Math.cos(theta) * size, 2)},${round(cy + Math.sin(theta) * size, 2)}`;
        })
        .join(' ');
      shape = {
        kind,
        points,
        opacity,
        fill,
      };
    }

    shapes.push({
      shape,
      motion,
    });
  }

  return shapes;
}

/**
 * 由文档标题派生页头氛围参数。同一标题与主题始终得到相同结果。
 * 背景线网与图形层使用独立盐值流，互不影响形态选择。
 */
export function deriveDocumentAtmosphere(
  title: string,
  theme: AtmosphereTheme = 'light',
): DocumentAtmosphere {
  const seed = hashValue(title);
  const hue = round(190 + unit(mix(seed, 1)) * 60, 1);
  const isDark = theme === 'dark';
  const gradientColor = `hsl(${hue} ${isDark ? 36 : 40}% ${isDark ? 48 : 62}% / ${isDark ? 0.16 : 0.12})`;
  const backdrop = deriveBackdrop(seed, hue, theme);

  return {
    hue,
    gradientColor,
    backdrop,
    backdropMotion:
      backdrop.kind === 'none' ? null : deriveBackdropMotion(seed),
    shapes: deriveShapes(seed, hue, theme),
  };
}

export function atmosphereArcPath(
  shape: Extract<AtmosphereShape, { kind: 'arc' }>,
) {
  return arcPath(shape.cx, shape.cy, shape.r, shape.startDeg, shape.sweepDeg);
}

/** 文档主题色相：主题明暗由各表面的 CSS 规则负责。 */
export function deriveDocumentThemeHue(title: string) {
  return deriveDocumentAtmosphere(title, 'light').hue;
}

/** 测试用：直接得到某一主题下的文档主题背景色字符串。 */
export function deriveDocumentThemeBackground(
  title: string,
  theme: AtmosphereTheme = 'light',
) {
  const hue = deriveDocumentThemeHue(title);
  const isDark = theme === 'dark';
  return `hsl(${hue} ${isDark ? 34 : 40}% ${isDark ? 28 : 94}% / ${isDark ? 0.42 : 0.48})`;
}

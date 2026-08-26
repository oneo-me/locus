import {
  ArrowUpIcon,
  BotIcon,
  BrainCircuitIcon,
  FolderIcon,
  GaugeIcon,
  GitBranchIcon,
  LaptopIcon,
  LockKeyholeIcon,
  SparklesIcon,
  WrenchIcon,
  type LucideIcon,
} from 'lucide-react';
import { Button } from '@locus/ui/components/button';
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupTextarea,
} from '@locus/ui/components/input-group';

export function AgentComposer() {
  return (
    <div className="w-full px-5 pb-2">
      <div className="mx-auto w-full max-w-[45rem]">
        <InputGroup>
          <InputGroupTextarea
            aria-label="给 Agent 发消息"
            className="min-h-12 max-h-48"
            placeholder="给 Agent 发消息…"
            readOnly
            rows={2}
          />
          <InputGroupAddon align="block-end" className="gap-1">
            <div className="flex min-w-0 flex-1 items-center gap-0.5 overflow-x-auto">
              <ComposerControl
                icon={SparklesIcon}
                label="选择模型"
                name="模型"
              />
              <ComposerControl
                icon={BrainCircuitIcon}
                label="推理 · 服务 · 上下文"
                name="模型特征"
              />
              <ComposerControl
                icon={BotIcon}
                label="标准模式"
                name="Agent 预设"
              />
              <ComposerControl
                icon={LockKeyholeIcon}
                label="监督"
                name="访问模式"
              />
              <ComposerControl icon={WrenchIcon} label="构建" name="交互模式" />
            </div>
            <InputGroupButton
              aria-disabled="true"
              aria-label="发送消息"
              className="rounded-full"
              size="icon-xs"
              tabIndex={-1}
              variant="default"
            >
              <ArrowUpIcon aria-hidden="true" />
            </InputGroupButton>
          </InputGroupAddon>
        </InputGroup>
        <div className="flex h-7 min-w-0 items-center gap-1 px-2 text-muted-foreground">
          <div className="flex min-w-0 flex-1 items-center gap-0.5 overflow-x-auto">
            <ComposerControl icon={FolderIcon} label="Locus" name="项目" />
            <ComposerControl icon={LaptopIcon} label="本地" name="工作位置" />
            <ComposerControl icon={GitBranchIcon} label="main" name="分支" />
          </div>
          <ComposerControl icon={GaugeIcon} label="上下文 0%" name="用量" />
        </div>
      </div>
    </div>
  );
}

function ComposerControl({
  icon: Icon,
  label,
  name,
}: {
  icon: LucideIcon;
  label: string;
  name: string;
}) {
  const content = (
    <>
      <Icon aria-hidden="true" data-icon="inline-start" />
      <span className="truncate">{label}</span>
    </>
  );
  const props = {
    'aria-disabled': true,
    'aria-label': `${name}：${label}`,
    className: 'max-w-40',
    tabIndex: -1,
  } as const;

  return (
    <Button {...props} size="nav" variant="ghost">
      {content}
    </Button>
  );
}

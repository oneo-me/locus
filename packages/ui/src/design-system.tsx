import FluentIconChevronRight from '@oneo/fluentui-icons-like-react/FluentIconChevronRight';
import FluentIconDocument from '@oneo/fluentui-icons-like-react/FluentIconDocument';
import FluentIconErrorCircle from '@oneo/fluentui-icons-like-react/FluentIconErrorCircle';
import FluentIconInfo from '@oneo/fluentui-icons-like-react/FluentIconInfo';
import FluentIconSearch from '@oneo/fluentui-icons-like-react/FluentIconSearch';
import type { ReactNode } from 'react';
import { Alert, AlertDescription, AlertTitle } from './components/alert';
import { Badge } from './components/badge';
import { Button, buttonVariants } from './components/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from './components/card';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from './components/collapsible';
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from './components/empty';
import { Input } from './components/input';
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
} from './components/input-group';
import { Kbd, KbdGroup } from './components/kbd';
import {
  Popover,
  PopoverContent,
  PopoverDescription,
  PopoverHeader,
  PopoverTitle,
  PopoverTrigger,
} from './components/popover';
import { Switch } from './components/switch';
import { Textarea } from './components/textarea';
import { cn } from './utils';

export function DesignSystemBrowser() {
  return (
    <main
      className="h-full min-h-0 min-w-0 overflow-x-hidden overflow-y-auto overscroll-contain bg-background text-sm"
      id="main-content"
    >
      <h1 className="sr-only">设计系统</h1>
      <div className="mx-auto w-[calc(100%-3rem)] max-w-[64rem] columns-2 gap-5 pt-8 pb-20">
        <ButtonPreview />
        <BadgePreview />
        <InputPreview />
        <TextareaPreview />
        <InputGroupPreview />
        <SwitchPreview />
        <KbdPreview />
        <AlertPreview />
        <EmptyPreview />
        <PopoverPreview />
        <CollapsiblePreview />
      </div>
    </main>
  );
}

function PreviewCard({
  children,
  description,
  title,
}: {
  children: ReactNode;
  description: string;
  title: string;
}) {
  return (
    <Card className="mb-5 break-inside-avoid">
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-5">{children}</CardContent>
    </Card>
  );
}

function DemoGroup({
  children,
  label,
}: {
  children: ReactNode;
  label: string;
}) {
  return (
    <div className="flex min-w-0 flex-col gap-2">
      <p className="text-xs text-muted-foreground">{label}</p>
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        {children}
      </div>
    </div>
  );
}

function ButtonPreview() {
  return (
    <PreviewCard
      description="主要操作、次要操作与导航状态。"
      title="Button 按钮"
    >
      <DemoGroup label="变体">
        <Button type="button">主要</Button>
        <Button type="button" variant="secondary">
          次要
        </Button>
        <Button type="button" variant="outline">
          描边
        </Button>
        <Button type="button" variant="ghost">
          幽灵
        </Button>
        <Button type="button" variant="destructive">
          危险
        </Button>
        <Button type="button" variant="link">
          链接
        </Button>
      </DemoGroup>
      <DemoGroup label="尺寸">
        <Button type="button" size="xs" variant="outline">
          极小
        </Button>
        <Button type="button" size="sm" variant="outline">
          较小
        </Button>
        <Button type="button" size="default" variant="outline">
          默认
        </Button>
        <Button type="button" size="lg" variant="outline">
          较大
        </Button>
      </DemoGroup>
      <DemoGroup label="图标尺寸">
        <Button aria-label="极小图标" size="icon-xs" variant="outline">
          <FluentIconSearch aria-hidden="true" title={null} />
        </Button>
        <Button aria-label="较小图标" size="icon-sm" variant="outline">
          <FluentIconSearch aria-hidden="true" title={null} />
        </Button>
        <Button aria-label="默认图标" size="icon" variant="outline">
          <FluentIconSearch aria-hidden="true" title={null} />
        </Button>
        <Button aria-label="较大图标" size="icon-lg" variant="outline">
          <FluentIconSearch aria-hidden="true" title={null} />
        </Button>
      </DemoGroup>
      <DemoGroup label="禁用">
        <Button disabled type="button">
          主要
        </Button>
        <Button disabled type="button" variant="outline">
          描边
        </Button>
      </DemoGroup>
    </PreviewCard>
  );
}

function BadgePreview() {
  return (
    <PreviewCard description="状态、标签与计数标记。" title="Badge 徽章">
      <DemoGroup label="变体">
        <Badge>默认</Badge>
        <Badge variant="secondary">次要</Badge>
        <Badge variant="destructive">危险</Badge>
        <Badge variant="outline">描边</Badge>
        <Badge variant="ghost">幽灵</Badge>
        <Badge variant="link">链接</Badge>
      </DemoGroup>
      <DemoGroup label="长文本截断">
        <Badge className="max-w-40" variant="secondary">
          <span className="truncate">一个非常长的标签名称会被截断</span>
        </Badge>
      </DemoGroup>
    </PreviewCard>
  );
}

function InputPreview() {
  return (
    <PreviewCard description="单行文本输入及其校验状态。" title="Input 输入框">
      <DemoGroup label="默认">
        <Input aria-label="示例输入" placeholder="请输入文档标题" />
      </DemoGroup>
      <DemoGroup label="禁用与无效">
        <Input
          aria-label="禁用输入"
          disabled
          placeholder="禁用状态"
          value="不可编辑"
          readOnly
        />
        <Input
          aria-invalid="true"
          aria-label="无效输入"
          defaultValue="非法的输入内容"
        />
      </DemoGroup>
    </PreviewCard>
  );
}

function TextareaPreview() {
  return (
    <PreviewCard
      description="多行文本输入，随内容自动伸展。"
      title="Textarea 多行输入"
    >
      <DemoGroup label="默认">
        <Textarea aria-label="示例多行输入" placeholder="请输入文档描述…" />
      </DemoGroup>
      <DemoGroup label="禁用">
        <Textarea aria-label="禁用多行输入" disabled placeholder="禁用状态" />
      </DemoGroup>
    </PreviewCard>
  );
}

function InputGroupPreview() {
  return (
    <PreviewCard
      description="输入框与图标、按钮、键盘提示的组合。"
      title="InputGroup 输入组"
    >
      <DemoGroup label="带图标">
        <InputGroup>
          <InputGroupAddon>
            <FluentIconSearch aria-hidden="true" title={null} />
          </InputGroupAddon>
          <InputGroupInput
            aria-label="搜索示例"
            placeholder="标题、标签或正文"
          />
        </InputGroup>
      </DemoGroup>
      <DemoGroup label="带键盘提示与按钮">
        <InputGroup>
          <InputGroupInput aria-label="命令示例" placeholder="输入命令…" />
          <InputGroupAddon align="inline-end">
            <Kbd>⌘K</Kbd>
          </InputGroupAddon>
        </InputGroup>
        <InputGroup>
          <InputGroupInput aria-label="带按钮的输入" placeholder="输入关键词" />
          <InputGroupAddon align="inline-end">
            <InputGroupButton type="button">搜索</InputGroupButton>
          </InputGroupAddon>
        </InputGroup>
      </DemoGroup>
    </PreviewCard>
  );
}

function SwitchPreview() {
  return (
    <PreviewCard description="二元开关，用于即时切换偏好。" title="Switch 开关">
      <DemoGroup label="状态">
        <Switch aria-label="关闭" />
        <Switch aria-label="开启" defaultChecked />
      </DemoGroup>
      <DemoGroup label="尺寸">
        <Switch aria-label="默认尺寸" defaultChecked />
        <Switch aria-label="小尺寸" defaultChecked size="sm" />
      </DemoGroup>
      <DemoGroup label="禁用">
        <Switch aria-label="禁用关闭" disabled />
        <Switch aria-label="禁用开启" defaultChecked disabled />
      </DemoGroup>
      <DemoGroup label="无效">
        <Switch aria-invalid aria-label="无效开关" defaultChecked />
      </DemoGroup>
    </PreviewCard>
  );
}

function KbdPreview() {
  return (
    <PreviewCard description="键盘按键与快捷键组合。" title="Kbd 键盘提示">
      <DemoGroup label="单键与组合">
        <Kbd>Esc</Kbd>
        <Kbd>Enter</Kbd>
        <KbdGroup>
          <Kbd>⌘</Kbd>
          <Kbd>K</Kbd>
        </KbdGroup>
        <KbdGroup>
          <Kbd>⇧</Kbd>
          <Kbd>⌘</Kbd>
          <Kbd>P</Kbd>
        </KbdGroup>
      </DemoGroup>
    </PreviewCard>
  );
}

function AlertPreview() {
  return (
    <PreviewCard description="页面内的状态与错误提示。" title="Alert 提示">
      <Alert>
        <FluentIconInfo aria-hidden="true" title={null} />
        <AlertTitle>同步已开启</AlertTitle>
        <AlertDescription>
          文档库变化会通过同源事件流自动推送到当前页面。
        </AlertDescription>
      </Alert>
      <Alert variant="destructive">
        <FluentIconErrorCircle aria-hidden="true" title={null} />
        <AlertTitle>无法保存元数据</AlertTitle>
        <AlertDescription>
          请检查 frontmatter 的 YAML 语法后重试。
        </AlertDescription>
      </Alert>
    </PreviewCard>
  );
}

function EmptyPreview() {
  return (
    <PreviewCard description="无内容时的引导状态。" title="Empty 空状态">
      <Empty className="border border-dashed">
        <EmptyHeader>
          <EmptyMedia variant="icon">
            <FluentIconDocument aria-hidden="true" title={null} />
          </EmptyMedia>
          <EmptyTitle>文档库还是空的</EmptyTitle>
          <EmptyDescription>
            在文档目录中加入 index.md，页面会自动建立索引。
          </EmptyDescription>
        </EmptyHeader>
        <EmptyContent>
          <Button size="sm" type="button" variant="outline">
            了解文档结构
          </Button>
        </EmptyContent>
      </Empty>
    </PreviewCard>
  );
}

function PopoverPreview() {
  return (
    <PreviewCard
      description="锚定触发的轻量浮层，经 Portal 渲染。"
      title="Popover 弹出层"
    >
      <DemoGroup label="点击触发">
        <Popover>
          <PopoverTrigger
            className={cn(buttonVariants({ variant: 'outline' }))}
          >
            打开弹出层
          </PopoverTrigger>
          <PopoverContent>
            <PopoverHeader>
              <PopoverTitle>弹出层标题</PopoverTitle>
              <PopoverDescription>
                用于承载轻量的补充信息与操作，不会被滚动容器裁切。
              </PopoverDescription>
            </PopoverHeader>
          </PopoverContent>
        </Popover>
      </DemoGroup>
    </PreviewCard>
  );
}

function CollapsiblePreview() {
  return (
    <PreviewCard
      description="可展开收起的渐进披露区域。"
      title="Collapsible 折叠面板"
    >
      <Collapsible defaultOpen>
        <CollapsibleTrigger
          className={cn(
            buttonVariants({ variant: 'outline' }),
            'data-[panel-open]:[&_svg]:rotate-90',
          )}
        >
          <FluentIconChevronRight
            data-icon="inline-start"
            aria-hidden="true"
            title={null}
          />
          切换详情
        </CollapsibleTrigger>
        <CollapsibleContent className="mt-2 rounded-lg border p-3 text-muted-foreground">
          折叠区域的内容，适合放置默认收起、按需展开的信息。
        </CollapsibleContent>
      </Collapsible>
    </PreviewCard>
  );
}

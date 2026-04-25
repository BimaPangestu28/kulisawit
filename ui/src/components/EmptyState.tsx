import { Card } from "@/components/ui/card";

interface Props {
  title: string;
  description?: string;
  children?: React.ReactNode;
}

export function EmptyState({ title, description, children }: Props) {
  return (
    <Card className="p-8 text-center">
      <div className="text-base font-medium">{title}</div>
      {description && (
        <div className="mt-2 text-sm text-muted-foreground">{description}</div>
      )}
      {children && <div className="mt-4">{children}</div>}
    </Card>
  );
}

import {
  Briefcase,
  ChartColumn,
  Code,
  Package,
  PenLine,
  type LucideIcon,
} from "lucide-react";

export const roleGroupIcons: Record<string, LucideIcon> = {
  product: Package,
  engineering: Code,
  data: ChartColumn,
  design_support: PenLine,
  business: Briefcase,
};

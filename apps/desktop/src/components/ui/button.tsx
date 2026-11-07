import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

// Primary is light-on-dark on purpose: white text on the accent green fails
// WCAG AA (see docs/GUI_MODERNIZATION.md §4.4).
const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 max-w-full whitespace-normal rounded-control text-body font-medium transition-[background-color,color,opacity] duration-(--duration-fast) ease-(--ease-out) disabled:pointer-events-none disabled:opacity-40 [&_svg]:size-4 [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        primary: "bg-text text-black hover:bg-white",
        secondary: "bg-surface-raised text-text hover:bg-[#3a3a3a]",
        ghost: "text-text-muted hover:bg-surface-raised hover:text-text",
        danger: "bg-danger/15 text-danger-text hover:bg-danger/25",
      },
      size: {
        sm: "min-h-8 px-3 py-1 text-small",
        md: "min-h-9 px-4 py-1.5",
        icon: "size-9",
      },
    },
    defaultVariants: { variant: "secondary", size: "md" },
  },
);

export function Button({
  className,
  variant,
  size,
  asChild = false,
  ...props
}: ComponentProps<"button"> & VariantProps<typeof buttonVariants> & { asChild?: boolean }) {
  const Comp = asChild ? Slot : "button";
  return <Comp className={cn(buttonVariants({ variant, size }), className)} {...props} />;
}
